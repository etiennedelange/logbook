//! Embedded assets: the app's own icons (the set from `ui/src/components/Icon.tsx`) and its fonts.
//! Anything else falls through to gpui-kit's bundled icons.

use gpui_kit::{AssetSource, Result, SharedString};
use std::borrow::Cow;

const ICONS: &[(&str, &[u8])] = &[
    ("icons/branch.svg", include_bytes!("../assets/icons/branch.svg")),
    ("icons/chat.svg", include_bytes!("../assets/icons/chat.svg")),
    ("icons/chevron-down.svg", include_bytes!("../assets/icons/chevron-down.svg")),
    ("icons/chevron-left.svg", include_bytes!("../assets/icons/chevron-left.svg")),
    ("icons/chevron-up.svg", include_bytes!("../assets/icons/chevron-up.svg")),
    ("icons/close.svg", include_bytes!("../assets/icons/close.svg")),
    ("icons/copy.svg", include_bytes!("../assets/icons/copy.svg")),
    ("icons/external.svg", include_bytes!("../assets/icons/external.svg")),
    ("icons/file.svg", include_bytes!("../assets/icons/file.svg")),
    ("icons/flame.svg", include_bytes!("../assets/icons/flame.svg")),
    ("icons/folder.svg", include_bytes!("../assets/icons/folder.svg")),
    ("icons/help.svg", include_bytes!("../assets/icons/help.svg")),
    ("icons/logbook.svg", include_bytes!("../assets/icons/logbook.svg")),
    ("icons/month.svg", include_bytes!("../assets/icons/month.svg")),
    ("icons/moon.svg", include_bytes!("../assets/icons/moon.svg")),
    ("icons/refresh.svg", include_bytes!("../assets/icons/refresh.svg")),
    ("icons/sessions.svg", include_bytes!("../assets/icons/sessions.svg")),
    ("icons/sparkle.svg", include_bytes!("../assets/icons/sparkle.svg")),
    ("icons/stats.svg", include_bytes!("../assets/icons/stats.svg")),
    ("icons/sun.svg", include_bytes!("../assets/icons/sun.svg")),
    ("icons/terminal.svg", include_bytes!("../assets/icons/terminal.svg")),
    ("icons/today.svg", include_bytes!("../assets/icons/today.svg")),
    ("icons/up.svg", include_bytes!("../assets/icons/up.svg")),
    ("icons/update.svg", include_bytes!("../assets/icons/update.svg")),
    ("icons/week.svg", include_bytes!("../assets/icons/week.svg")),
    ("icons/win-close.svg", include_bytes!("../assets/icons/win-close.svg")),
    ("icons/win-max.svg", include_bytes!("../assets/icons/win-max.svg")),
    ("icons/win-min.svg", include_bytes!("../assets/icons/win-min.svg")),
    ("icons/win-restore.svg", include_bytes!("../assets/icons/win-restore.svg")),
];

pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        if let Some((_, bytes)) = ICONS.iter().find(|(p, _)| *p == path) {
            return Ok(Some(Cow::Borrowed(*bytes)));
        }
        gpui_kit::assets::Assets.load(path)
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let mut out: Vec<SharedString> = ICONS.iter().filter(|(p, _)| p.starts_with(path)).map(|(p, _)| (*p).into()).collect();
        out.extend(gpui_kit::assets::Assets.list(path)?);
        Ok(out)
    }
}

/// IBM Plex Sans and JetBrains Mono (both SIL OFL; licences are in `assets/fonts`), as the web UI uses.
pub fn fonts() -> Vec<Cow<'static, [u8]>> {
    vec![
        Cow::Borrowed(include_bytes!("../assets/fonts/IBMPlexSans-Regular.ttf").as_slice()),
        Cow::Borrowed(include_bytes!("../assets/fonts/IBMPlexSans-Medium.ttf").as_slice()),
        Cow::Borrowed(include_bytes!("../assets/fonts/IBMPlexSans-SemiBold.ttf").as_slice()),
        Cow::Borrowed(include_bytes!("../assets/fonts/JetBrainsMono-Regular.ttf").as_slice()),
        Cow::Borrowed(include_bytes!("../assets/fonts/JetBrainsMono-Medium.ttf").as_slice()),
        Cow::Borrowed(include_bytes!("../assets/fonts/JetBrainsMono-SemiBold.ttf").as_slice()),
    ]
}
