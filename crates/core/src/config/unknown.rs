//! Finding the configuration keys this build does not implement.
//!
//! Serde drops a key the struct does not declare, and it does so without a
//! word. A repository that writes a section the tool does not read is then
//! left believing that the section does something. The estate writes
//! eighteen sections; this build reads a handful of them.
//!
//! The report is what closes the gap. It names every key in the file that
//! this build does not act on, so the gap is visible rather than assumed
//! away. It never fails a command: a repository with a section the tool
//! does not read should still be able to run the tool.

use serde_json::{Map, Value};

use super::schema::ConfigSchema;

/// A key the file may use that the schema declares under another name.
///
/// The alias table has to be kept in step with the `serde` aliases in the
/// schema, because the report compares the names in the file with the
/// names in the schema. A new alias in the schema without a line here
/// would be reported as an unknown key, which is a false alarm.
const ALIASES: &[(&str, &str, &str)] = &[
    // (section, the name in the file, the name in the schema)
    ("secrets", "patterns", "additional_patterns"),
];

/// A subtree whose keys cannot be enumerated.
///
/// The pre-push timeouts are keyed by the name of the check, so the schema
/// cannot list them. The names the runner knows are `lint`, `build` and
/// `test`; a name that is not one of those is reported by the runner
/// itself, which is where the list of checks lives.
const OPEN_SUBTREES: &[&str] = &["pre_push.timeouts"];

/// The keys in `content` that this build does not implement.
///
/// The two trees are compared by name. A key the schema declares is known,
/// whatever its value; a key it does not declare is reported.
///
/// # Errors
///
/// Returns an error when the content cannot be parsed as TOML.
pub fn unknown_keys_in(content: &str, schema: &ConfigSchema) -> anyhow::Result<Vec<String>> {
    let raw: toml::Value = toml::from_str(content)?;
    let raw = serde_json::to_value(raw)?;
    let known = serde_json::to_value(schema)?;
    Ok(unknown_keys(&raw, &known))
}

/// The keys in `raw` that the schema in `known` does not declare.
#[must_use]
pub fn unknown_keys(raw: &Value, known: &Value) -> Vec<String> {
    let mut unknown = Vec::new();
    if let (Some(raw), Some(known)) = (raw.as_object(), known.as_object()) {
        walk(raw, known, "", &mut unknown);
    }
    unknown.sort();
    unknown
}

/// Compare one table of the file with the matching table of the schema.
fn walk(
    raw: &Map<String, Value>,
    known: &Map<String, Value>,
    path: &str,
    unknown: &mut Vec<String>,
) {
    for (key, raw_value) in raw {
        let full_path = if path.is_empty() {
            key.clone()
        } else {
            format!("{path}.{key}")
        };

        let lookup = alias_for(path, key).unwrap_or(key.as_str());
        let Some(known_value) = known.get(lookup) else {
            // The whole subtree is unknown. Naming the section is more
            // useful than naming every key inside it, and the section is
            // what a repository would have to remove to silence the report.
            unknown.push(full_path);
            continue;
        };

        if OPEN_SUBTREES.contains(&full_path.as_str()) {
            continue;
        }

        if let (Some(raw_table), Some(known_table)) =
            (raw_value.as_object(), known_value.as_object())
        {
            walk(raw_table, known_table, &full_path, unknown);
        }
    }
}

/// The schema name for a key the file writes under another name.
fn alias_for(section: &str, key: &str) -> Option<&'static str> {
    ALIASES
        .iter()
        .find(|(name, alias, _)| *name == section && *alias == key)
        .map(|(_, _, canonical)| *canonical)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys(content: &str) -> Vec<String> {
        let schema: ConfigSchema = toml::from_str(content).expect("the configuration should parse");
        unknown_keys_in(content, &schema).expect("the configuration should be inspectable")
    }

    #[test]
    fn test_a_known_section_is_not_reported() {
        let unknown = keys("[general]\nproject_name = \"FoodShare\"\n");
        assert_eq!(unknown, Vec::<String>::new());
    }

    #[test]
    fn test_an_unknown_section_is_reported_by_its_name() {
        let unknown = keys("[docs]\noutput_dir = \"docs/api\"\n");
        assert_eq!(unknown, vec!["docs".to_string()]);
    }

    #[test]
    fn test_an_unknown_key_in_a_known_section_is_reported() {
        let unknown = keys("[general]\nsomething_new = true\n");
        assert_eq!(unknown, vec!["general.something_new".to_string()]);
    }

    #[test]
    fn test_a_section_with_no_consumer_is_reported_whole() {
        // The report names the section, not each key in it: a repository
        // that wants to silence it removes the section.
        let unknown = keys("[android]\nmodule = \"app\"\nbuild_variant = \"debug\"\n");
        assert_eq!(unknown, vec!["android".to_string()]);
    }

    #[test]
    fn test_an_alias_is_not_reported() {
        // `patterns` is the name the estate writes; the schema calls it
        // `additional_patterns`. The alias table is what keeps the report
        // from crying wolf about it.
        let unknown = keys("[secrets]\npatterns = [\"a-secret-shape\"]\n");
        assert!(unknown.is_empty(), "the alias was reported: {unknown:?}");
    }

    #[test]
    fn test_the_canonical_name_is_not_reported_either() {
        let unknown = keys("[secrets]\nadditional_patterns = [\"a-secret-shape\"]\n");
        assert!(unknown.is_empty(), "the key was reported: {unknown:?}");
    }

    #[test]
    fn test_an_open_subtree_is_not_reported() {
        // The timeouts are keyed by the name of the check, so the schema
        // cannot list them.
        let unknown = keys("[pre_push.timeouts]\nvalidate = 120\nbuild = 1800\n");
        assert!(
            unknown.is_empty(),
            "the timeouts were reported: {unknown:?}"
        );
    }

    #[test]
    fn test_a_nested_table_is_walked() {
        let unknown = keys("[pre_push]\nfail_fast = true\n\n[pre_push.guidance]\nenabled = true\n");
        assert_eq!(unknown, vec!["pre_push.guidance".to_string()]);
    }

    #[test]
    fn test_the_report_is_sorted() {
        let unknown = keys("[zeta]\na = 1\n\n[alpha]\nb = 2\n");
        assert_eq!(unknown, vec!["alpha".to_string(), "zeta".to_string()]);
    }
}
