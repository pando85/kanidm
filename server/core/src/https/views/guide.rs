use askama::Template;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GuidedUiMode {
    Off,
    Subtle,
    Full,
}

impl GuidedUiMode {
    fn from_value(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" | "full" => Self::Full,
            "subtle" => Self::Subtle,
            "0" | "false" | "no" | "off" | "" => Self::Off,
            _ => Self::Off,
        }
    }
}

pub(crate) fn guided_ui_mode() -> GuidedUiMode {
    std::env::var("KUBIDM_GUIDED_UI")
        .ok()
        .map(|value| GuidedUiMode::from_value(&value))
        .unwrap_or(GuidedUiMode::Off)
}

pub(crate) fn guided_ui_enabled() -> bool {
    guided_ui_mode() != GuidedUiMode::Off
}

pub(crate) fn guided_ui_full() -> bool {
    guided_ui_mode() == GuidedUiMode::Full
}

pub(crate) fn guided_ui_subtle() -> bool {
    guided_ui_mode() == GuidedUiMode::Subtle
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GuidedMotionMode {
    Auto,
    Full,
    Reduced,
    Static,
}

impl GuidedMotionMode {
    fn from_value(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "full" => Self::Full,
            "reduced" => Self::Reduced,
            "static" => Self::Static,
            "auto" | "" => Self::Auto,
            _ => Self::Auto,
        }
    }
}

impl fmt::Display for GuidedMotionMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Auto => "auto",
            Self::Full => "full",
            Self::Reduced => "reduced",
            Self::Static => "static",
        })
    }
}

pub(crate) fn guided_motion_mode() -> GuidedMotionMode {
    std::env::var("KUBIDM_GUIDED_MOTION")
        .ok()
        .map(|value| GuidedMotionMode::from_value(&value))
        .unwrap_or(GuidedMotionMode::Auto)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GuideDialogVariant {
    Orient,
    Teach,
    Suggest,
    Celebrate,
}

impl fmt::Display for GuideDialogVariant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Orient => "orient",
            Self::Teach => "teach",
            Self::Suggest => "suggest",
            Self::Celebrate => "celebrate",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GuideRecommendation {
    None,
    Required,
    Recommended,
    WorksOk,
    Optional,
}

impl GuideRecommendation {
    fn label(self) -> &'static str {
        match self {
            Self::None => "",
            Self::Required => "Required",
            Self::Recommended => "Recommended",
            Self::WorksOk => "Works OK",
            Self::Optional => "Optional",
        }
    }
}

impl fmt::Display for GuideRecommendation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::None => "none",
            Self::Required => "required",
            Self::Recommended => "recommended",
            Self::WorksOk => "works_ok",
            Self::Optional => "optional",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GuideSeverity {
    Neutral,
    Positive,
    Caution,
    Critical,
}

impl GuideSeverity {
    fn is_alert(self) -> bool {
        matches!(self, Self::Caution | Self::Critical)
    }
}

impl fmt::Display for GuideSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Neutral => "neutral",
            Self::Positive => "positive",
            Self::Caution => "caution",
            Self::Critical => "critical",
        })
    }
}

#[derive(Template)]
#[template(path = "guide/crab_dialog.html")]
pub(crate) struct CrabDialogView {
    variant: GuideDialogVariant,
    title: Option<String>,
    text: String,
}

impl CrabDialogView {
    pub(crate) fn new(
        variant: GuideDialogVariant,
        title: Option<impl Into<String>>,
        text: impl Into<String>,
    ) -> Self {
        Self {
            variant,
            title: title.map(Into::into),
            text: text.into(),
        }
    }
}

#[derive(Template)]
#[template(path = "guide/recommendation_option.html")]
pub(crate) struct RecommendationOptionView {
    recommendation: String,
    recommendation_label: &'static str,
    title: String,
    reason: String,
    disabled: bool,
}

impl RecommendationOptionView {
    pub(crate) fn new(
        recommendation: GuideRecommendation,
        title: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            recommendation: recommendation.to_string(),
            recommendation_label: recommendation.label(),
            title: title.into(),
            reason: reason.into(),
            disabled: false,
        }
    }

    pub(crate) fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

#[derive(Template)]
#[template(path = "guide/security_notice.html")]
pub(crate) struct SecurityNoticeView {
    severity: String,
    is_alert: bool,
    title: Option<String>,
    text: String,
}

impl SecurityNoticeView {
    pub(crate) fn new(
        severity: GuideSeverity,
        title: Option<impl Into<String>>,
        text: impl Into<String>,
    ) -> Self {
        Self {
            severity: severity.to_string(),
            is_alert: severity.is_alert(),
            title: title.map(Into::into),
            text: text.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct JourneyProgressItem {
    label: String,
    detail: Option<String>,
    complete: bool,
}

impl JourneyProgressItem {
    pub(crate) fn new(
        label: impl Into<String>,
        detail: Option<impl Into<String>>,
        complete: bool,
    ) -> Self {
        Self {
            label: label.into(),
            detail: detail.map(Into::into),
            complete,
        }
    }
}

#[derive(Template)]
#[template(path = "guide/journey_progress.html")]
pub(crate) struct JourneyProgressView {
    items: Vec<JourneyProgressItem>,
}

impl JourneyProgressView {
    pub(crate) fn new(items: Vec<JourneyProgressItem>) -> Self {
        Self { items }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guide_mode_parser_is_backward_compatible_and_bounded() {
        assert_eq!(GuidedUiMode::from_value("1"), GuidedUiMode::Full);
        assert_eq!(GuidedUiMode::from_value("TRUE"), GuidedUiMode::Full);
        assert_eq!(GuidedUiMode::from_value("full"), GuidedUiMode::Full);
        assert_eq!(GuidedUiMode::from_value("subtle"), GuidedUiMode::Subtle);
        assert_eq!(GuidedUiMode::from_value("off"), GuidedUiMode::Off);
        assert_eq!(GuidedUiMode::from_value("unexpected"), GuidedUiMode::Off);
    }

    #[test]
    fn motion_mode_parser_is_bounded_and_safe_by_default() {
        assert_eq!(GuidedMotionMode::from_value("full"), GuidedMotionMode::Full);
        assert_eq!(GuidedMotionMode::from_value("REDUCED"), GuidedMotionMode::Reduced);
        assert_eq!(GuidedMotionMode::from_value("static"), GuidedMotionMode::Static);
        assert_eq!(GuidedMotionMode::from_value("auto"), GuidedMotionMode::Auto);
        assert_eq!(GuidedMotionMode::from_value("unexpected"), GuidedMotionMode::Auto);
    }

    #[test]
    fn crab_dialog_renders_accessible_text() {
        let html = CrabDialogView::new(
            GuideDialogVariant::Teach,
            Some("Why a passkey?"),
            "The private key is not sent to Kubidm during authentication.",
        )
        .render()
        .expect("dialog should render");

        assert!(html.contains("data-variant=\"teach\""));
        assert!(html.contains("Why a passkey?"));
        assert!(html.contains("private key"));
    }

    #[test]
    fn valid_alternative_is_not_rendered_as_warning() {
        let html = RecommendationOptionView::new(
            GuideRecommendation::WorksOk,
            "Use a password",
            "Valid for this account and policy.",
        )
        .render()
        .expect("recommendation should render");

        assert!(html.contains("data-recommendation=\"works_ok\""));
        assert!(html.contains("Works OK"));
        assert!(!html.contains("data-severity=\"caution\""));
    }

    #[test]
    fn caution_notice_is_authoritative_alert_ui() {
        let html = SecurityNoticeView::new(
            GuideSeverity::Caution,
            Some("Action required"),
            "Your organisation requires another security step.",
        )
        .render()
        .expect("notice should render");

        assert!(html.contains("data-severity=\"caution\""));
        assert!(html.contains("role=\"alert\""));
    }

    #[test]
    fn journey_progress_is_semantic_not_a_score() {
        let html = JourneyProgressView::new(vec![
            JourneyProgressItem::new("You can sign in", None::<String>, true),
            JourneyProgressItem::new("Recovery ready", Some("Optional next step"), false),
        ])
        .render()
        .expect("progress should render");

        assert!(html.contains("Identity setup progress"));
        assert!(html.contains("data-complete=\"true\""));
        assert!(html.contains("data-complete=\"false\""));
        assert!(!html.contains("score"));
    }

    #[test]
    fn all_variants_are_stable_strings() {
        assert_eq!(GuideDialogVariant::Orient.to_string(), "orient");
        assert_eq!(GuideDialogVariant::Celebrate.to_string(), "celebrate");
        assert_eq!(GuideRecommendation::Required.to_string(), "required");
        assert_eq!(GuideRecommendation::WorksOk.to_string(), "works_ok");
        assert_eq!(GuideSeverity::Critical.to_string(), "critical");
        assert_eq!(GuidedMotionMode::Static.to_string(), "static");
    }
}
