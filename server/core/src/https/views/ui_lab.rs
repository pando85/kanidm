use askama::Template;
use askama_web::WebTemplate;

#[derive(Template, WebTemplate)]
#[template(path = "ui_lab.html")]
pub(super) struct UiLabView;

pub(super) async fn view_lab_get() -> UiLabView {
    UiLabView
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ui_lab_template_renders_story_harness() {
        let html = UiLabView.render().expect("UI Lab template should render");

        assert!(html.contains("data-ui-lab"));
        assert!(html.contains("data-story=\"first-login\""));
        assert!(html.contains("ui-lab-preview"));
    }
}
