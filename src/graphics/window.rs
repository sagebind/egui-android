use ndk::native_window::NativeWindow;
use raw_window_handle::{
    AndroidNdkWindowHandle, DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle,
    RawWindowHandle, WindowHandle,
};

pub(crate) fn as_raw_window_handle(native_window: &NativeWindow) -> RawWindowHandle {
    RawWindowHandle::from(AndroidNdkWindowHandle::new(native_window.ptr().cast()))
}

pub(crate) struct AndroidSurfaceTarget {
    native_window: NativeWindow,
}

impl AndroidSurfaceTarget {
    pub fn new(native_window: NativeWindow) -> Self {
        Self { native_window }
    }
}

impl HasWindowHandle for AndroidSurfaceTarget {
    fn window_handle(&self) -> Result<WindowHandle, HandleError> {
        let raw_window_handle = as_raw_window_handle(&self.native_window);

        unsafe { Ok(WindowHandle::borrow_raw(raw_window_handle)) }
    }
}

impl HasDisplayHandle for AndroidSurfaceTarget {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        Ok(DisplayHandle::android())
    }
}
