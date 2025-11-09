/*
 * Watermarker
 *
 *  Copyright (C) 2025 Kuwagata HIROSHI <kgt9221@gmail.com>
 */

//!
//! コンフィギュレーションファイル関連の処理をまとめたモジュール
//!

use std::str::FromStr;
use std::path::Path;
use std::path::PathBuf;

use anyhow::Result;
use serde::{Deserialize, Deserializer};

use super::Position;
use super::Resolution;

///
/// デシリアライズ用の&strからenumへの変換の為の中継関数
///
fn from_str<'de, D, T>(deserializer: D)
    -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: FromStr,
   <T as FromStr>::Err: std::fmt::Display,
{
    match Option::<String>::deserialize(deserializer)? {
        Some(s) => {
            T::from_str(&s).map(Some).map_err(serde::de::Error::custom)
        }
        None => Ok(None),
    }
}

///
/// コンフィギュレーションデータを集約する構造体
///
#[derive(Debug, Deserialize)]
pub(super) struct Config {
    /// ロゴ関連の設定情報の格納先
    logo: Option<LogoInfo>,

    /// レンダリング関連の設定情報の格納先
    output: Option<OutputInfo>,
}

impl Config {
    //
    // ロゴで使用するファイルへのパスへのアクセサ
    //
    pub(super) fn logo_file_path(&self) -> Option<PathBuf> {
        self.logo
            .as_ref()
            .and_then(|logo| logo.file_path.as_ref())
            .cloned()
    }

    ///
    /// ロゴの展開位置へのアクセサ
    ///
    pub(super) fn logo_position(&self) -> Option<Position> {
        self.logo
            .as_ref()
            .and_then(|logo| logo.position.as_ref())
            .cloned()
    }

    ///
    /// 出力解像度へのアクセサ
    ///
    pub(super) fn output_resolution(&self) -> Option<Resolution> {
        self.output
            .as_ref()
            .and_then(|output| output.resolution.as_ref())
            .cloned()
    }

    ///
    /// 出力先へのアクセサ
    ///
    pub(super) fn output_path(&self) -> Option<PathBuf> {
        self.output
            .as_ref()
            .and_then(|output| output.output_path.as_ref())
            .cloned()
    }
}

///
/// ロゴ関連の設定を格納する構造体
///
#[derive(Debug, Deserialize)]
pub struct LogoInfo {
    /// ロゴに使用する画像ファイル(PNG)へのパス
    file_path: Option<PathBuf>,

    /// ロゴを配置する場所
    position: Option<Position>,
}

///
/// レンダリング設定を格納する構造体
///
#[derive(Debug, Deserialize)]
pub struct OutputInfo {
    /// 出力解像度(プリセット名またはWxH形式)
    #[serde(deserialize_with = "from_str")]
    resolution: Option<Resolution>,

    /// 出力先
    output_path: Option<PathBuf>,
}

///
/// コンフィギュレーションファイルの読み込み
///
pub(super) fn read<P>(path: P) -> Result<Config>
where 
    P: AsRef<Path>
{
     Ok(toml::from_str(&std::fs::read_to_string(path)?)?)
}
