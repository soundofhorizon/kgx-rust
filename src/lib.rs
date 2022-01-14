pub mod utils;
pub mod commands;
pub mod loops;
#[macro_use]
extern crate diesel;
pub mod schema;
pub mod models;


use std::collections::HashMap;
use regex::Regex;

pub fn stack_check_fn(value: &str) -> f32 {

    // [a lc + b st + c…]などがvalueで来ることを想定する(正しくない文字列が渡されればNoneを返す)
    // 小数で来た場合、小数で計算して最後にintぐるみをして値を返す
    // :param value: [a lc + b st + c…]の形の価格
    // :return: 価格をn個にしたもの(小数は丸め込む)。またはNone

    let unit_definition = HashMap::from([
        ("lc", 3456.0),
        ("st", 64.0),
    ]); //単位と対応する値

    // 最初にマッチした箇所全体を取り出す例
    let re = Regex::new(r"\s*\d+((\.\d+)?(st|lc))?(\s*\+\s*\d+((\.\d+)?(st|lc))?)*\s*").unwrap();
    let value_lowercase = value.to_lowercase();
    let caps = re.captures(&value_lowercase).unwrap();
    let re_caps = caps.get(0).unwrap().as_str();

    // 空白文字を削除
    let space_re = Regex::new(r"\s").unwrap();
    let remove_space_re_caps = space_re.replace_all(re_caps, "");

    // Unitの定義に従い、st, lcが検出されたら乗算を行い、なければそのまま足す
    let mut result: f32 = 0.0;
    for term in remove_space_re_caps.split("+"){
        let unit_match = Regex::new(r"(st|lc)?$").unwrap().captures(term);
        let unit = unit_match.unwrap().get(0).unwrap().as_str();
        let unit_convert = unit_definition.get(unit);
        if let Some(unit_value) = unit_convert {
            result += term.replace(unit, "").parse::<f32>().unwrap() * unit_value;
        } else {
            result += term.replace(unit, "").parse::<f32>().unwrap()
        }
    }

    result
}