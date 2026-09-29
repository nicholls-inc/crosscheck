//! E2E test for the TypeScript frontend: assertion sites (`<runtime read> as T`)
//! become `writes_to` override edges into the slots of `T`.

mod test_helpers;

use std::collections::BTreeSet;
use std::fs;

use tempfile::TempDir;
use test_helpers::*;

const STASH: &str = "export interface Stash {
  clientName: string;
  origin: 'login' | 'signup';
  search?: string;
}

export function peek(raw: string, clientName: string): Stash | null {
  const stash = JSON.parse(raw) as Stash;
  return stash.clientName === clientName ? stash : null;
}
";

const THEME: &str = "export type Theme = 'light' | 'dark';

export function loadTheme(): Theme {
  return localStorage.getItem('theme') as Theme;
}
";

fn typescript_db() -> (TempDir, rusqlite::Connection) {
    let tmp = TempDir::new().unwrap();
    let app = tmp.path().join("app");
    fs::create_dir_all(app.join("src")).unwrap();
    fs::write(app.join("src/stash.ts"), STASH).unwrap();
    fs::write(app.join("src/theme.ts"), THEME).unwrap();
    let conn = extract_to_db(&app, &tmp);
    (tmp, conn)
}

#[test]
fn test_typescript_assertion_sites() {
    let (_tmp, conn) = typescript_db();

    let nodes: BTreeSet<(String, String)> = ["function", "model"]
        .iter()
        .flat_map(|kind| query_nodes(&conn, kind))
        .map(|n| (n.name, n.kind))
        .collect();
    let expected: BTreeSet<(String, String)> = [
        ("Stash.clientName", "model"),
        ("Stash.origin", "model"),
        ("Theme", "model"),
        ("loadTheme", "function"),
        ("peek", "function"),
    ]
    .iter()
    .map(|(n, k)| (n.to_string(), k.to_string()))
    .collect();
    assert_eq!(nodes, expected, "`search?: string` rejects nothing, so it is no slot");
    assert_eq!(count_rows(&conn, "nodes"), 5);

    let edges = query_edges(&conn);
    let overrides = edges
        .iter()
        .filter(|e| e.relationship == "writes_to" && e.source_override)
        .count();
    assert_eq!((overrides, edges.len()), (3, 3));

    assert_eq!(
        node_rows(&conn, "Stash.origin", "precondition"),
        vec![r#"choices=["login","signup"]"#, "nullability=0"]
    );
    assert_eq!(
        override_rows(&conn, &edges, "peek", "Stash.clientName", "writes_to", None),
        vec!["nullability=0"],
        "the guard against the `clientName: string` parameter"
    );
    assert_eq!(
        override_rows(&conn, &edges, "peek", "Stash.origin", "writes_to", None),
        Vec::<String>::new(),
        "JSON.parse guarantees nothing"
    );
    assert_eq!(
        override_rows(&conn, &edges, "loadTheme", "Theme", "writes_to", None),
        vec!["nullability=1"],
        "getItem may return null"
    );
    assert_eq!(the_edge(&edges, "loadTheme", "Theme", "writes_to", None).site_line, Some(4));
}
