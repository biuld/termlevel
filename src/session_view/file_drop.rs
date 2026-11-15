//! Convert native local-file URLs to literal shell arguments. No file contents
//! or image bytes cross the UI boundary, and a drop never submits the line.
use url::Url;

pub(super) fn paste_paths(uris: &[String]) -> Option<String> {
    if uris.is_empty() {
        return None;
    }
    let mut text = String::new();
    for uri in uris {
        let url = Url::parse(uri).ok()?;
        if url.query().is_some() || url.fragment().is_some() {
            return None;
        }
        let path = url.to_file_path().ok()?;
        let path = path.to_str()?;
        // Newlines/control bytes can become input actions in unbracketed TUIs.
        // Reject the whole drop rather than silently altering a filename.
        if !path.starts_with('/') || path.chars().any(char::is_control) {
            return None;
        }
        text.push('\'');
        text.push_str(&path.replace('\'', "'\\''"));
        text.push_str("' ");
        if text.len() > 1024 * 1024 {
            return None;
        }
    }
    Some(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_arguments_preserve_unicode_order_and_shell_metacharacters() {
        let paths = ["/tmp/截图 你好.png", "/tmp/a'b$(touch nope);`x`.jpg"];
        let uris = paths.map(|path| Url::from_file_path(path).unwrap().to_string());
        assert_eq!(
            paste_paths(&uris).unwrap(),
            "'/tmp/截图 你好.png' '/tmp/a'\\''b$(touch nope);`x`.jpg' "
        );
        assert_eq!(
            paste_paths(&["file:///tmp/a%2520b.png".into()]).unwrap(),
            "'/tmp/a%20b.png' "
        );
    }

    #[test]
    fn rejects_nonlocal_urls_controls_and_mixed_invalid_drops() {
        assert!(paste_paths(&[]).is_none());
        for bad in [
            "https://example.com/image.png",
            "file://remote/tmp/a.png",
            "file:///tmp/a%0Ab.png",
            "file:///tmp/a%00.png",
            "file:///tmp/a.png?x=1",
            "file:///tmp/a.png#fragment",
        ] {
            assert!(
                paste_paths(&["file:///tmp/good.png".into(), bad.into()]).is_none(),
                "{bad}"
            );
        }
    }
}
