use std::collections::HashMap;
use std::sync::OnceLock;

pub(crate) fn sanitize_language(raw: &str) -> &'static str {
    match raw.trim() {
        "fr" => "fr",
        _ => "en",
    }
}

fn french_catalog() -> &'static HashMap<String, String> {
    static CATALOG: OnceLock<HashMap<String, String>> = OnceLock::new();
    CATALOG.get_or_init(|| {
        serde_json::from_str(include_str!("../../ui/static/locales/fr.json"))
            .expect("Bundled French translation catalog must be valid JSON")
    })
}

/// Translate one server-inserted interface label. Unknown labels and languages
/// fall back to the English text, so callers can pass any label safely.
pub(crate) fn translate<'a>(raw_language: &str, label: &'a str) -> &'a str {
    if sanitize_language(raw_language) != "fr" {
        return label;
    }
    french_catalog()
        .get(label)
        .map(String::as_str)
        .unwrap_or(label)
}

/// Translate explicit template markers before application/user data is inserted.
/// Login labels are server-rendered so cached or disabled JavaScript cannot leave
/// the sign-in form in a different language from the dashboard.
pub(crate) fn render_template(template: &str, raw_language: &str) -> String {
    let language = sanitize_language(raw_language);
    let messages = if language == "fr" {
        french_catalog().clone()
    } else {
        HashMap::new()
    };
    let mut html = String::with_capacity(template.len());
    let mut remaining = template;
    while let Some(start) = remaining.find("{{t:") {
        html.push_str(&remaining[..start]);
        let tail = &remaining[start + 4..];
        let Some(end) = tail.find("}}") else {
            html.push_str(&remaining[start..]);
            remaining = "";
            break;
        };
        let key = &tail[..end];
        let translation = messages.get(key).map(String::as_str).unwrap_or(key);
        html.push_str(&crate::templates::escape_html(translation));
        remaining = &tail[end + 2..];
    }
    html.push_str(remaining);
    let config = serde_json::json!({
        "language": language,
        "messages": messages,
    })
    .to_string()
    .replace('<', "\\u003c")
    .replace('>', "\\u003e")
    .replace('&', "\\u0026");
    html.replace("{{ui_language}}", language)
        .replace(
            "{{ui_language_en_selected}}",
            if language == "en" { "selected" } else { "" },
        )
        .replace(
            "{{ui_language_fr_selected}}",
            if language == "fr" { "selected" } else { "" },
        )
        .replace(
            "<!-- I18N_CONFIG -->",
            &format!(r#"<script id="amud-i18n-config" type="application/json">{config}</script>"#),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn english_is_default_and_invalid_language_cannot_inject_markup() {
        let template = r#"<html lang="{{ui_language}}"><h2>{{t:Sign In}}</h2>"#;
        for language in ["", "en", "de", "fr\"><script>"] {
            let rendered = render_template(template, language);
            assert_eq!(rendered, r#"<html lang="en"><h2>Sign In</h2>"#);
        }
    }

    #[test]
    fn french_login_is_translated_without_javascript() {
        let html = render_template(include_str!("../../ui/templates/login.html"), "fr");
        assert!(html.contains(r#"lang="fr""#));
        assert!(html.contains("Se connecter"));
        assert!(html.contains("Nom d’utilisateur"));
        assert!(html.contains("Mot de passe"));
        assert!(html.contains("Revenir au tableau de bord comme invité"));
        assert!(!html.contains("{{t:"));
        assert!(!html.contains(">Sign In<"));
    }

    #[test]
    fn language_selector_and_catalog_match_saved_language() {
        let template = r#"<option value="en" {{ui_language_en_selected}}>English</option><option value="fr" {{ui_language_fr_selected}}>Français</option><!-- I18N_CONFIG -->"#;
        let html = render_template(template, "fr");
        assert!(html.contains(r#"value="fr" selected"#));
        let json = html.split(r#"type="application/json">"#).nth(1).unwrap();
        let json = json.split("</script>").next().unwrap();
        let config: serde_json::Value = serde_json::from_str(json).unwrap();
        assert_eq!(config["language"], "fr");
        assert_eq!(config["messages"]["Sign In"], "Se connecter");
    }

    #[test]
    fn page_title_labels_are_translated_on_the_server() {
        assert_eq!(translate("fr", "DASHBOARD"), "TABLEAU DE BORD");
        assert_eq!(translate("fr", "FEEDS"), "ACTUALITÉS");
        assert_eq!(translate("en", "DASHBOARD"), "DASHBOARD");
        assert_eq!(translate("de", "FEEDS"), "FEEDS");
        assert_eq!(translate("fr", "My Dashboard"), "My Dashboard");
    }

    #[test]
    fn settings_titles_keep_the_application_name() {
        let html = render_template(include_str!("../../ui/templates/settings.html"), "fr");
        assert!(html.contains("<title>Paramètres - {{app_name}}</title>"));
        assert!(html.contains("<h2>{{app_name}} Paramètres</h2>"));
    }

    #[test]
    fn empty_dashboard_hint_keeps_both_steps() {
        let hint = translate(
            "fr",
            "Log in as Admin and click \"Add App\" to register your infrastructure.",
        );
        assert!(hint.contains("administrateur"));
        assert!(hint.contains(translate("fr", "Add App")));
        assert!(hint.contains("infrastructure"));
    }

    #[test]
    fn only_explicit_markers_are_translated() {
        let html = render_template("<p>All</p><p>{{t:All}}</p><p>{{username}}</p>", "fr");
        assert_eq!(html, "<p>All</p><p>Tout</p><p>{{username}}</p>");
    }
}
