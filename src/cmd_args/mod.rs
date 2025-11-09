/*
 * Watermarker
 *
 *  Copyright (C) 2025 Kuwagata HIROSHI <kgt9221@gmail.com>
 */

//!
//! コマンドラインオプション関連の処理をまとめたモジュール
//!

mod config;

use std::fmt::Display;
use std::sync::Arc;
use std::str::FromStr;
use std::path::PathBuf;

use anyhow::{anyhow, Result};
use clap::{Parser, ValueEnum};
use directories::BaseDirs;
use image::RgbaImage;
use serde::Deserialize;

///
/// デフォルトのコンフィグレーションファイルのパス情報を生成
///
/// # 戻り値
/// コンフィギュレーションファイルのパス情報
///
fn default_config_path() -> PathBuf {
    BaseDirs::new()
        .unwrap()
        .config_local_dir()
        .join(env!("CARGO_PKG_NAME"))
        .join("config.toml")
}

///
/// ロゴ画像の配置位置
///
#[derive(Debug, Clone, Copy, PartialEq, ValueEnum, Deserialize)]
#[clap(rename_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING-KEBAB-CASE")]
pub enum Position {
    /// 左上
    TopLeft,

    /// 右上
    TopRight,

    /// 左下
    BottomLeft,

    /// 右下
    BottomRight,

    /// 画像中央
    Center,
}

// Displayトレイトの実装
impl Display for Position {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self {
            Self::TopLeft => "TOP-LEFT",
            Self::TopRight => "TOP-RIGHT",
            Self::BottomLeft => "BOTTOM-LEFT",
            Self::BottomRight => "BOTTOM-RIGHT",
            Self::Center => "CENTER",
        })
    }
}

///
/// プリセット解像度の定義
///
pub enum PresetResolution {
    /// QVGA (320x240)
    QVGA,

    /// VGA (640x480)
    VGA,

    /// SVGA (800x600)
    SVGA,

    /// HD (1280x720)
    HD,

    /// QuadVGA (1280x960)
    QuadVGA,

    /// FullHD (1920x1080)
    FullHD,
}

// FromStrトレイトの実装
impl FromStr for PresetResolution {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "qvga" => Ok(PresetResolution::QVGA),
            "vga" => Ok(PresetResolution::VGA),
            "svga" => Ok(PresetResolution::SVGA),
            "hd" => Ok(PresetResolution::HD),
            "quadvga" => Ok(PresetResolution::QuadVGA),
            "fullhd" => Ok(PresetResolution::FullHD),
            _ => Err(format!("該当する解像度無し:{}", s)),
        }
    }
}

// Intoトレイトの実装
impl Into<Resolution> for PresetResolution {
    fn into(self) -> Resolution {
        match self {
            PresetResolution::QVGA => Resolution::new(320, 240),
            PresetResolution::VGA => Resolution::new(640, 480),
            PresetResolution::SVGA => Resolution::new(800, 600),
            PresetResolution::HD => Resolution::new(1280, 720),
            PresetResolution::QuadVGA => Resolution::new(1280, 960),
            PresetResolution::FullHD => Resolution::new(1920, 1080),
        }
    }
}

///
/// 解像度を管理する構造体
///
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct Resolution {
    /// 画像の幅(ピクセル数)
    width: u32,

    /// 画像の高さ(ピクセル数)
    height: u32,
}

// FromStrトレイトの実装
impl FromStr for Resolution {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        /*
         * プリセット解像度としての評価
         */
        if let Ok(preset) = PresetResolution::from_str(s) {
            return Ok(preset.into());
        }

        /*
         * 数値形式(WxH)としての評価
         */
        let parts: Vec<&str> = s.split('x').collect();

        if parts.len() != 2 {
            return Err(format!("解像度形式が不正: {}", s));
        }

        let width = parts[0]
            .parse::<u32>()
            .map_err(|_| format!("幅の指定が無効: {}", parts[0]))?;

        let height = parts[1]
            .parse::<u32>()
            .map_err(|_| format!("高さの指定が無効: {}", parts[0]))?;

        Ok(Self {width, height})
    }
}

// Displayトレイトの実装
impl Display for Resolution {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}×{}", self.width, self.height)
    }
}

/// Resolutionに対する実装
impl Resolution {
    ///
    /// オブジェクトの生成
    ///
    /// # 引数
    /// * `width` - ターゲット解像度の幅(ピクセル数)
    /// * `height` - ターゲット解像度の高さ(ピクセル数)
    ///
    /// # 戻り値
    /// 生成したオブジェクトを返す
    ///
    fn new(width: u32, height: u32) -> Self {
        Self {width, height }
    }

    ///
    /// スケール比の算出
    ///
    /// # 引数
    /// * `width` - 変換元画像の幅(ピクセル数)
    /// * `height` - 変換元画像の高さ(ピクセル数)
    ///
    /// # 戻り値
    /// アスペクト比を維持したまま目標解像度と同等の画素数の座椅子に調整するた
    /// めの比率を返す。
    /// 引数 `width`及び`height`にこの関数の戻り値を掛けると`self`の持つ解像度
    /// と同等の面積を持つ矩形にリサイズできる。
    ///
    pub fn scale_ratio(&self, width: u32, height: u32) -> f32 {
        ((self.width * self.height) as f32 / (width * height) as f32).sqrt()
    }

    ///
    /// 指定スケールでの出力サイズの算出
    ///
    /// # 引数
    /// * `width` - 変換元画像の幅(ピクセル数)
    /// * `height` - 変換元画像の高さ(ピクセル数)
    ///
    /// # 戻り値
    /// 引数 `width`と`height`を`self`が持つ解像度と同等の画素数を持つ画像の幅
    /// と高さに変換し、その幅と高さをパックしたタプルを返す。
    ///
    pub fn scaled_size(&self, width: u32, height: u32) -> (u32, u32) {
        let scale = self.scale_ratio(width, height);

        (
            (width as f32 * scale).round() as u32,
            (height as f32 * scale).round() as u32
        )
    }
}

///
/// コマンドラインオプションの情報をまとめる構造体
///
#[derive(Parser, Debug, Clone)]
#[command(
    name = "watermarker",
    about = "画像に透かしロゴを埋め込むCLIツール",
    version,
    long_about = None,
)]
pub struct Options {
    /// コンフィギュレーションファイルのパス
    #[arg(short = 'c', long = "config-file", value_name = "FILE")]
    config_file: Option<PathBuf>,

    /// 出力先ディレクトリ
    #[arg(short = 'o', long = "output-path", value_name= "PATH")]
    output_path: Option<PathBuf>,

    /// ロゴとして使用する透過PNGファイルのパス
    #[arg(short = 'l', long = "logo-file-path", value_name = "PATH")]
    logo_file_path: Option<PathBuf>,

    /// ロゴの配置位置
    #[arg(short = 'p', long = "logo-position", value_enum,
        value_name = "POSITION")]
    logo_position: Option<Position>,

    /// 出力解像度(プリセット名またはWxH形式)
    ///
    /// 使用例:
    ///   -r HD
    ///   -r 1280x720
    #[arg(short = 'r', long = "resolution", default_value = "HD")]
    resolution: Option<Resolution>,

    /// 上書き許可
    #[arg(short = 'f', long, default_value = "false")]
    force: bool,

    /// 設定情報の表示
    #[arg(short = 's', long = "show-options", default_value = "false")]
    show_options: bool,

    /// 入力ファイルまたはディレクトリ
    #[arg(required = true)]
    inputs: Vec<PathBuf>,

    #[arg(skip)]
    logo_image: Option<RgbaImage>,
}

impl Options {
    ///
    /// 出力フォルダへのアクセサ
    ///
    pub(crate) fn output_path(&self) -> PathBuf {
        if let Some(path) = &self.output_path {
            path.clone()
        } else {
            PathBuf::from(".")
        }
    }

    ///
    /// ロゴファイルへのアクセサ
    ///
    /// # 注記
    /// バリデーション関数により、self.logo_file_path がNoneのままこの関数が呼
    /// ばれることが無いことが保証されている。
    ///
    pub(crate) fn logo_file_path(&self) -> PathBuf {
        self.logo_file_path.as_ref().unwrap().clone()
    }

    ///
    /// ロゴイメージへのアクセサ
    ///
    /// # 注記
    /// バリデーション関数により、self.logo_image がNoneのままこの関数が呼ばれ
    /// ることが無いことが保証されている。
    ///
    pub(crate) fn logo_image(&self) -> &RgbaImage {
        self.logo_image.as_ref().unwrap()
    }

    ///
    /// ロゴ展開位置へのアクセサ
    ///
    pub(crate) fn logo_position(&self) -> Position {
        if let Some(position) = self.logo_position {
            position
        } else {
            Position::BottomRight
        }
    }

    ///
    /// 出力解像度へのアクセサ
    ///
    pub(crate) fn resolution(&self) -> Resolution {
        if let Some(resolution) = self.resolution {
            resolution
        } else {
            PresetResolution::HD.into()
        }
    }

    ///
    /// 強制書き込み可否のフラグへのアクセサ
    ///
    pub(crate) fn is_force(&self) -> bool {
        self.force
    }

    ///
    /// 入力ファイルリストへのアクセサ
    ///
    pub(crate) fn inputs(&self) -> Vec<PathBuf> {
        self.inputs.clone()
    }

    ///
    /// オプション情報モードか否かのフラグへのアクセサ
    ///
    /// # 戻り値
    /// オプション情報表示モードが指定されている場合は`true`が、通常モードのが
    /// 指定されている場合は`false`が返される。
    ///
    pub(crate) fn is_show_options(&self) -> bool {
        self.show_options
    }

    ///
    /// オプション設定内容の表示
    ///
    pub(crate) fn show_options(&self) {
        let config_path = if let Some(path) = &self.config_file {
            Some(path.clone())
        } else {
            let path = default_config_path();

            if path.exists() {
                Some(path)
            } else {
                None
            }
        };

        println!("config path:       {:?}", config_path);
        println!("output path:       {:?}", self.output_path());
        println!("logo file path:    {:?}", self.logo_file_path());
        println!("logo position:     {}", self.logo_position());
        println!("output resolution: {}", self.resolution());
    }
    ///
    /// コンフィギュレーションの適用
    /// 
    /// # 注記
    /// config.tomlを読み込みオプション情報に反映する。
    ///
    fn apply_config(&mut self) -> Result<()> {
        let path = if let Some(path) = &self.config_file {
            // オプションでコンフィギュレーションファイルのパスが指定されて
            // いる場合、そのパスに何もなければエラー
            if !path.exists() {
                return Err(anyhow!("{} is not exists", path.display()));
            }

            // 指定されたパスを返す
            path.clone()
        } else {
            // 指定されていない場合はデフォルトのパスを返す
            default_config_path()
        };

        // この時点でパスに何も無い場合はそのまま何もせず正常終了
        if !path.exists() {
            return Ok(());
        }

        // 指定されたパスにあるのがファイルでなければエラー
        if !path.is_file() {
            return Err(anyhow!("{} is not file", path.display()));
        }

        // そのパスからコンフィギュレーションを読み取る
        match config::read(&path) {
            // 読み取れた場合は内容を適用
            Ok(config) => {
                if self.logo_file_path.is_none() {
                    if let Some(path) = &config.logo_file_path() {
                        self.logo_file_path = Some(path.clone());
                    }
                }

                if self.logo_position.is_none() {
                    if let Some(position) = config.logo_position() {
                        self.logo_position = Some(position);
                    }
                }

                if self.resolution.is_none() {
                    if let Some(resolution) = &config.output_resolution() {
                        self.resolution = Some(resolution.clone());
                    }
                }

                if self.output_path.is_none() {
                    if let Some(path) = &config.output_path() {
                        self.output_path = Some(path.clone());
                    }
                }

                Ok(())
            }

            // エラーが出たらエラー
            Err(err) => Err(anyhow!("{}", err))
        }
    }

    ///
    /// 設定情報のバリデーションとキャッシュの構築
    ///
    /// # 戻り値
    /// 設定情報に問題が無い場合は`Ok(())`を返す。問題があった場合はエラー情報
    /// を`Err()`でラップして返す。
    ///
    fn validate(&mut self) -> Result<()> {
        /*
         * 出力先パスの確認
         */
        if let Some(path) = &self.output_path {
            if !path.is_dir() {
                return Err(anyhow!(
                    "output path \"{}\" is not directory",
                    path.display()
                ));
            }
        }

        /*
         * ロゴファイルのパスの確認
         */
        if let Some(path) = &self.logo_file_path {
            if !path.is_file() {
                return Err(anyhow!(
                    "logo file path \"{}\" is not file",
                    path.display()
                ));
            }
        } else {
            return Err(anyhow!("logo file path is not specified"));
        }

        /*
         * 入力ファイルまたはディレクトリの確認
         */
        for path in self.inputs.iter() {
            if !(path.is_file() || path.is_dir()) {
                return Err(anyhow!(
                    "input path \"{}\" is not file or directory",
                    path.display()
                ));
            }
        }

        /*
         * ロゴファイルの読み込み
         */
        self.logo_image = Some(
            image::open(self.logo_file_path())?.to_rgba8()
        );

        Ok(())
    }
}

///
/// コマンドライン引数のパース
///
/// # 戻り値
/// 処理に成功した場合はオプション設定をパックしたオブジェクトを`Ok()`でラップ
/// して返す。失敗した場合はエラー情報を`Err()`でラップして返す。
///
pub(crate) fn parse() -> Result<Arc<Options>> {
    let mut opts = Options::parse();

    /*
     * コンフィギュレーションファイルの適用
     */
    opts.apply_config()?;

    /*
     * 設定情報のバリデーションとキャッシュの構築
     */
    opts.validate()?;

    /*
     * 設定情報の返却
     */
    Ok(Arc::new(opts))
}
