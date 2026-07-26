use askama::Template;
use askama_web::WebTemplate;

#[derive(Template, WebTemplate)]
#[template(path = "ui_lab.html")]
pub(super) struct UiLabView {
    title: &'static str,
}

pub(super) async fn view_lab_get() -> UiLabView {
    UiLabView {
        title: "Kubidm UI Lab",
    }
}
