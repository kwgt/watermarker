/*
 * Watermarker
 *
 *  Copyright (C) 2025 Hiroshi KUWAGATA <kgt9221@gmail.com>
 */

//!
//! プログラムのエントリポイント
//!

mod cmd_args;

use std::fs::File;
use std::io::{BufWriter, BufReader};
use std::path::Path;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use fast_image_resize::{
    FilterType, PixelType, Resizer, ResizeOptions, ResizeAlg
};
use fast_image_resize::images::Image;
use image::{ImageBuffer, RgbaImage};
use image::imageops::overlay;
use mozjpeg::{ColorSpace, Compress, Decompress};
use walkdir::{DirEntry, WalkDir};

use cmd_args::Options;

///
/// プログラムのエントリポイント
///
fn main() {
    /*
     * コマンドラインオプションのパース
     */
    let opts = match cmd_args::parse() {
        Ok(opts) => opts,
        Err(err) => {
            eprintln!("error: {}", err);
            std::process::exit(1);
        },
    };

    if opts.is_show_options() {
        opts.show_options();
        std::process::exit(0);
    }

    /*
     * 実行関数の呼び出し
     */
    if let Err(err) = run(opts) {
        eprintln!("error: {}", err);
        std::process::exit(1);
    }
}

///
/// プログラムの実行関数
///
/// # 引数
/// * `opts` - オプション情報をパックしたオブジェクト
///
/// # 戻り値
/// プログラムが正常狩猟した場合は、`Ok(())`を返す。失敗した場合はエラー情報を
/// `Err()`でラップして返す。
///
fn run(opts: Arc<Options>) -> Result<()> {
    for path in opts.inputs().iter() {
        if path.is_file() {
            /*
             * ファイルの場合はそのまま処理
             */
            proc_file(&opts, &path)?;

        } else if path.is_dir() {
            /*
             * ディレクトリの場合は、再帰的にJPEGファイルを探査しそれぞれで
             * 処理する
             */
            for entry in jpeg_files(path) {
                proc_file(&opts, entry.path())?;
            }
        }
    }

    Ok(())
}

///
/// JPEGファイルのリストアップ
///
/// # 引数
/// * `path` - 探査の起点となるフォルダへのパス
///
/// # 戻り値
/// JPEGファイルをリストアップしたイテレーター
///
/// # 注記
/// 引数で指定されたフォルダを起点に再帰的に降下探査しJPEGファイルをリストアッ
/// プするイテレータを返す。
///
fn jpeg_files<P>(path: P) -> impl Iterator<Item = DirEntry>
where 
    P: AsRef<Path>,
{
    WalkDir::new(path)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .filter(|e| {
            e.path()
                .extension()
                .and_then(|s| s.to_str())
                .map(|ext| {
                    match ext.to_lowercase().as_str() {
                        "jpg" | "jpeg" => true,
                        _ => false
                    }
                })
                .unwrap_or(false)
        })
}

///
/// JPEGファイルに対する画像操作
///
/// # 引数
/// * `opts` - オプション情報をパックしたオブジェクト
/// * `input_path` - 処理対象のJPEGファイルへのパス
///
/// # 戻り値
/// 処理に成功した場合は`Ok(())`を返す。処理に失敗した場合はエラー情報を`Err()`
/// でラップして返す。
///
/// # 注記
/// オプション情報で強制書き込みが指定されていない場合かつ、出力ファイルが既に
/// 存在する場合は処理をスキップした上で`Ok(())`を返すので注意すること。
///
fn proc_file<P>(opts: &Arc<Options>, input_path: P) -> Result<()>
where 
    P: AsRef<Path>
{
    let input_path = input_path.as_ref();
    let output_path = opts.output_path()
        .join(input_path.file_name().unwrap());

    /*
     * 出力ファイルが既に存在する場合はスキップ
     */
    if output_path.exists() && !opts.is_force() {
        eprintln!(
            "{} => {} skip (already exist)",
            input_path.display(),
            output_path.display()
        );
        return Ok(());
    }

    /*
     * JPEGのデコード
     */
    let image = decode_jpeg(input_path)?;

    /*
     * 画像のリサイズ
     */
    let (width, height) = opts.resolution()
        .scaled_size(image.width(), image.height());

    let mut bg = resize_image(width, height, image)?;

    /*
     * ロゴの重畳
     */
    let logo = opts.logo_image();
    let right = width as i64 - logo.width() as i64;
    let bottom = height as i64 - logo.height() as i64;

    let (x, y) = match opts.logo_position() {
        cmd_args::Position::TopLeft => (0, 0),
        cmd_args::Position::TopRight => (right, 0),
        cmd_args::Position::BottomLeft => (0, bottom),
        cmd_args::Position::BottomRight => (right, bottom),
        cmd_args::Position::Center => (right / 2, bottom / 2),
    };

    overlay(&mut bg, logo, x, y);

    /*
     * ファイルの書き込み
     */
    encode_jpeg(&output_path, bg)?;

    println!("{} => {}", input_path.display(), output_path.display());

    Ok(())
}

///
/// JPEGファイルのデコード
///
/// # 引数
/// * `path` - デコード対象のJPEGファイルへのパス
///
/// # 戻り値
/// 処理に成功した場合はデコードした画像を`RgbaImage`オブエクトとして`Ok()`で
/// ラップして返す。失敗した場合はエラー情報を`Err()`でラップして返す。
///
fn decode_jpeg<P>(path: P) -> Result<RgbaImage>
where 
    P: AsRef<Path>
{
    //let mut file = File::open(path)?;
    //let mut jpeg = Vec::new();
    //
    //file.read_to_end(&mut jpeg)?;
    let reader= BufReader::new(File::open(path)?);

    let mut decomp = Decompress::new_reader(reader)?.rgba()?;

    let width = decomp.width() as u32;
    let height = decomp.height() as u32;
    let pixels = decomp.read_scanlines::<[u8; 4]>()?.concat();

    let image: RgbaImage = ImageBuffer::from_raw(width, height, pixels)
        .ok_or_else(|| anyhow!("invalid dimensions"))?;

    Ok(image)
}

///
/// JPEGファイルへのエンコード(ファイルへの出力)
///
/// # 引数
/// * `path` - エンコード結果の書き込み対象ファイルへのパス
/// * `image` - エンコード対象のイメージデータ
///
/// # 戻り値
/// 処理に成功した場合は`Ok(())`を返す。失敗した場合はエラー情報を `Err()`でラ
/// ップして返す。
///
fn encode_jpeg<P>(path: P, image: RgbaImage) -> Result<()>
where 
    P: AsRef<Path>
{
    let writer = BufWriter::new(File::create(path)?);

    let mut comp = Compress::new(ColorSpace::JCS_EXT_RGBA);
    comp.set_size(image.width() as usize, image.height() as usize);
    comp.set_quality(90 as f32);
    comp.set_optimize_coding(true);

    let mut comp = comp.start_compress(writer)?;
    comp.write_scanlines(image.as_raw().as_slice())?;
    comp.finish()?;

    Ok(())
}

///
/// 画像データのリサイズ　
///
/// # 引数
/// * `width` - ターゲットサイズの幅(ピクセル数)
/// * `height` - ターゲットサイズの高さ(ピクセル数) 
/// * `image` - リサイズ元の画像データ
///
/// # 戻り値
/// リサイズに成功した場合は、リサイズされた画像データを`Ok()`でラップして返す。
/// 処理に失敗した場合はエラー情報を`Err()`でラップして返す。
///
fn resize_image(width: u32, height: u32, image: RgbaImage)
    -> Result<RgbaImage>
{
    let mut src = Image::from_vec_u8(
        image.width(),
        image.height(),
        image.into_raw(),
        PixelType::U8x4
    )?;

    let mut dst = Image::new(width, height, PixelType::U8x4);

    let mut resizer = Resizer::new();
    let resize_opts = ResizeOptions::new()
        .resize_alg(ResizeAlg::Convolution(FilterType::Lanczos3));

    resizer.resize(&mut src, &mut dst, &resize_opts)?;

    Ok(RgbaImage::from_raw(width, height, dst.into_vec()).unwrap())
}
