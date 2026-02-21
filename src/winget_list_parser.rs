use crate::winget::{PackageEntry, Source};

pub fn parse_package_entries(output: &str) -> Vec<PackageEntry> {
    let mut entries = Vec::new();

    let lines: Vec<&str> = output.lines().collect();
    if lines.is_empty() {
        return entries;
    }

    let start_idx = find_data_start(&lines);

    for line in lines.into_iter().skip(start_idx) {
        if line.trim().is_empty() {
            continue;
        }

        if let Some(entry) = parse_line_to_entry(line) {
            entries.push(entry);
        }
    }

    entries
}

fn find_data_start(lines: &[&str]) -> usize {
    lines
        .iter()
        .position(|l| {
            let t = l.trim();
            !t.is_empty() && t.chars().all(|c| c == '-')
        })
        .map(|i| i + 1)
        .unwrap_or(0)
}

fn parse_line_to_entry(line: &str) -> Option<PackageEntry> {
    let source_str = line.split_whitespace().last()?;
    let source = source_str.parse::<Source>().ok()?;
    let before_source = line.rfind(source_str).map(|i| &line[..i])?;

    let toks: Vec<&str> = before_source.split_whitespace().collect();
    let id_token = toks.iter().rev().find(|t| is_id_token(t)).copied()?;

    let name = extract_name_from_line(line, before_source, id_token);

    let id_pos = before_source.rfind(id_token).unwrap();
    let after_id = &before_source[id_pos + id_token.len()..];
    let after_tokens: Vec<&str> = after_id.split_whitespace().collect();
    let update_available = after_tokens.len() >= 2;

    Some(PackageEntry {
        source,
        id: id_token.to_string(),
        name,
        update_available,
    })
}

fn is_version_like(tok: &str) -> bool {
    let mut chars = tok.chars();
    match chars.next() {
        Some(c) if c.is_ascii_digit() => tok
            .chars()
            .all(|ch| ch.is_ascii_digit() || ch == '.' || ch == '…'),
        Some('v') | Some('V') => {
            let rest: String = chars.collect();
            rest.chars()
                .next()
                .map(|ch| ch.is_ascii_digit())
                .unwrap_or(false)
                && rest
                    .chars()
                    .all(|ch| ch.is_ascii_digit() || ch == '.' || ch == '…')
        }
        _ => false,
    }
}

fn is_id_token(tok: &str) -> bool {
    if is_version_like(tok) {
        return false;
    }

    if tok.contains('\\') {
        return true;
    }

    if tok.contains('.') && tok.chars().any(|c| c.is_alphabetic()) {
        return true;
    }

    tokens_has_uppercase_alpha(tok)
}

fn tokens_has_uppercase_alpha(tok: &str) -> bool {
    tok.chars().any(|c| c.is_ascii_uppercase())
        && tok
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
}

fn extract_name_from_line(line: &str, before_source: &str, id_token: &str) -> String {
    let id_pos = before_source.rfind(id_token).unwrap();
    line[..id_pos].trim_end().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_package_entries() {
        let entries = parse_package_entries(
            r"Name                                            Id                                        Version    Available  Source
-----------------------------------------------------------------------------------------------------------------------
PowerToys (Preview) x64                         Microsoft.PowerToys                       0.97.1     0.97.2     winget
Microsoft Visual C++ 2010  x64 Redistributable… Microsoft.VCRedist.2010.x64               10.0.40219            winget
PowerShell                                      9MZ1SNWT0N5D                              7.5.4.0               msstore
Windows Notepad                                 MSIX\Microsoft.WindowsNotepad_11.2510.14… 11.2510.1…
Windows ターミナル                              Microsoft.WindowsTerminal                 1.23.2021…            winget
MSYS2 64bit                                     MSYS2.MSYS2                               20220603   20251213   winget
",
        );
        assert_eq!(entries[0].name, "PowerToys (Preview) x64".to_string());
        assert_eq!(entries[0].id, "Microsoft.PowerToys");
        assert_eq!(entries[0].source, Source::Winget);
        assert!(entries[0].update_available);

        assert_eq!(
            entries[1].name,
            "Microsoft Visual C++ 2010  x64 Redistributable…".to_string()
        );
        assert_eq!(entries[1].id, "Microsoft.VCRedist.2010.x64");
        assert_eq!(entries[1].source, Source::Winget);
        assert!(!entries[1].update_available);

        assert_eq!(entries[2].name, "PowerShell".to_string());
        assert_eq!(entries[2].id, "9MZ1SNWT0N5D");
        assert_eq!(entries[2].source, Source::MsStore);
        assert!(!entries[2].update_available);

        assert_eq!(entries[3].name, "Windows ターミナル".to_string());
        assert_eq!(entries[3].id, "Microsoft.WindowsTerminal");
        assert_eq!(entries[3].source, Source::Winget);
        assert!(!entries[3].update_available);

        assert_eq!(entries[4].name, "MSYS2 64bit".to_string());
        assert_eq!(entries[4].id, "MSYS2.MSYS2");
        assert_eq!(entries[4].source, Source::Winget);
        assert!(entries[4].update_available);
    }
}
