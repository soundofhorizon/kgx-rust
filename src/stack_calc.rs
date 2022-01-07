use std::collections::HashMap;
use regex::Regex;

pub fn stack_check(value: &str) -> &str{

    // [a lc + b st + c…]などがvalueで来ることを想定する(正しくない文字列が渡されればNoneを返す)
    // 小数で来た場合、小数で計算して最後にintぐるみをして値を返す
    // :param value: [a lc + b st + c…]の形の価格
    // :return: 価格をn個にしたもの(小数は丸め込む)。またはNone

    UNITS = Hashmap::from([
        ("lc", 3456),
        ("st", 64)
    ]); //単位と対応する値

    // 最初にマッチした箇所全体を取り出す例
    let re = Regex::new(r"\s*\d+((\.\d+)?(st|lc))?(\s*\+\s*\d+((\.\d+)?(st|lc))?)*\s*").unwrap();
    let caps = re.captures(value).unwrap();
    return caps.at(0).unwrap();
}