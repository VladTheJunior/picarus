use gpui_kit::SharedString;

pub trait EnumNameExt {
    fn title(&self) -> SharedString;
}
