pub fn render_hidden_ids(name: &str, ids: &[String]) -> String {
    let joined = ids.join(",");
    format!(
        r#"<input type="hidden" name="{name}" value="{value}">"#,
        name = html_escape::encode_text(name),
        value = html_escape::encode_text(&joined),
    )
}
