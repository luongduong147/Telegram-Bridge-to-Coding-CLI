use regex::Regex;

pub fn markdown_to_html(s: &str) -> String {
    let s = s.replace('&', "&amp;")
             .replace('<', "&lt;")
             .replace('>', "&gt;");

    let re = Regex::new(r"(?ms)```\w*\n(.+?)```").unwrap();
    let s = re.replace_all(&s, "<pre>$1</pre>").to_string();

    let re = Regex::new(r"`([^`]+)`").unwrap();
    let s = re.replace_all(&s, "<code>$1</code>").to_string();

    let re = Regex::new(r"\*\*(.+?)\*\*").unwrap();
    let s = re.replace_all(&s, "<b>$1</b>").to_string();

    s
}
