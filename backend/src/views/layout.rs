use minijinja::{Environment, context};
use std::sync::OnceLock;

pub struct Breadcrumb {
    pub label: String,
    pub href: Option<String>,
}

pub struct NavItem {
    pub label: String,
    pub href: String,
    pub key: String,
}

pub fn breadcrumb(label: &str, href: Option<String>) -> Breadcrumb {
    Breadcrumb {
        label: label.to_string(),
        href,
    }
}

pub fn render_layout(title: &str, active: &str, breadcrumbs: Vec<Breadcrumb>, content: &str) -> String {
    let nav_items = vec![
        NavItem { label: "Dashboard".to_string(), href: "/".to_string(), key: "dashboard".to_string() },
        NavItem { label: "Server / Profile".to_string(), href: "/server".to_string(), key: "server".to_string() },
        NavItem { label: "Pakete / Mods".to_string(), href: "/packages".to_string(), key: "packages".to_string() },
        NavItem { label: "Run / Logs".to_string(), href: "/run-logs".to_string(), key: "run".to_string() },
        NavItem { label: "Settings".to_string(), href: "/settings".to_string(), key: "settings".to_string() },
    ];

    let env = template_env();
    let context = context! {
        title => title,
        active => active,
        nav_items => nav_items,
        breadcrumbs => breadcrumbs,
        content => content,
    };

    env.get_template("layouts/base.html")
        .and_then(|template| template.render(context))
        .unwrap_or_else(|err| format!("Template error: {err}"))
}

pub fn template_env() -> &'static Environment<'static> {
    static ENV: OnceLock<Environment<'static>> = OnceLock::new();
    ENV.get_or_init(|| {
        let mut env = Environment::new();
        env.set_loader(minijinja::path_loader(templates_dir()));
        env.set_auto_escape_callback(|_| minijinja::AutoEscape::Html);
        env
    })
}

fn templates_dir() -> String {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("templates")
        .to_string_lossy()
        .to_string()
}
