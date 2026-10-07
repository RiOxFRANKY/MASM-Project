use super::cp437::to_unicode;
use super::palette::colors;
use super::Frame;
use std::fmt::Write;

fn css((red, green, blue): (u8, u8, u8)) -> String {
    format!("#{red:02x}{green:02x}{blue:02x}")
}

pub fn html(frame: &Frame) -> String {
    let mut page = String::from(
        "<!doctype html><html><head><meta charset=\"utf-8\"><title>DOSBox-RS screen</title>\
         <style>body{background:#000;margin:16px}pre{font:16px/1 Consolas,'Courier New',monospace;margin:0}</style>\
         </head><body><pre>",
    );
    for row in 0..frame.rows {
        for column in 0..frame.columns {
            let index = (row * frame.columns + column) * 2;
            let (foreground, background) = colors(frame.cells[index + 1], frame.blink);
            let character = match to_unicode(frame.cells[index]) {
                '<' => "&lt;".to_string(),
                '>' => "&gt;".to_string(),
                '&' => "&amp;".to_string(),
                other => other.to_string(),
            };
            let _ = write!(
                page,
                "<span style=\"color:{};background:{}\">{}</span>",
                css(foreground),
                css(background),
                character
            );
        }
        page.push('\n');
    }
    page.push_str("</pre></body></html>\n");
    page
}
