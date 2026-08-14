use gpui::SharedString;

pub trait EnumNameExt {
    fn title(&self) -> SharedString;
}
