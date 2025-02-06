use android_activity::{
    input::{
        Axis, InputEvent, KeyAction, KeyEvent, MetaState, MotionAction, MotionEvent, Pointer,
        ToolType,
    },
    InputStatus,
};
use egui::{
    pos2, vec2, Event, Modifiers, MouseWheelUnit, PointerButton, TouchDeviceId, TouchId, TouchPhase,
};

/// Stateful object that processes input events from Android, and translates
/// them into egui input events.
pub(crate) struct InputHandler {}

impl InputHandler {
    pub fn new() -> Self {
        Self {}
    }

    /// Process an input event.
    ///
    /// If the input event warrants events to be passed into egui, then
    /// `receiver` will be invoked with such events as an argument. An Android
    /// input event could result in zero, one, or multiple egui events.
    ///
    /// The return value indicates whether the event was understood and
    /// processed, or if it was an unknown event type.
    pub fn process(
        &mut self,
        android_event: &InputEvent,
        pixels_per_point: f32,
        mut receiver: impl FnMut(Event),
    ) -> InputStatus {
        log::debug!("Processing input event: {:?}", android_event);

        match android_event {
            InputEvent::KeyEvent(key_event) => {
                if let Some(event) = to_egui_key_event(key_event) {
                    receiver(event);
                    InputStatus::Handled
                } else {
                    InputStatus::Unhandled
                }
            }

            InputEvent::MotionEvent(motion_event) => {
                match motion_event.action() {
                    MotionAction::Scroll => {
                        for pointer in motion_event.pointers() {
                            receiver(Event::MouseWheel {
                                delta: vec2(
                                    pointer.axis_value(Axis::Hscroll),
                                    pointer.axis_value(Axis::Vscroll),
                                ) / pixels_per_point,
                                modifiers: modifiers_from_meta_state(motion_event.meta_state()),
                                unit: MouseWheelUnit::Point,
                            });
                        }

                        InputStatus::Handled
                    }

                    MotionAction::PointerDown => {
                        // Event::PointerButton { pos: (), button: (), pressed: (), modifiers: () };
                        todo!()
                    }

                    MotionAction::Down => {
                        for pointer in motion_event.pointers() {
                            receiver(create_touch_event(
                                motion_event,
                                &pointer,
                                TouchPhase::Start,
                                pixels_per_point,
                            ));

                            receiver(create_click_event(
                                motion_event,
                                &pointer,
                                true,
                                pixels_per_point,
                            ));
                        }

                        InputStatus::Handled
                    }

                    MotionAction::Up => {
                        for pointer in motion_event.pointers() {
                            receiver(create_touch_event(
                                motion_event,
                                &pointer,
                                TouchPhase::End,
                                pixels_per_point,
                            ));

                            receiver(create_click_event(
                                motion_event,
                                &pointer,
                                false,
                                pixels_per_point,
                            ));
                        }

                        if motion_event.pointer_count() == 0 {
                            receiver(Event::PointerGone);
                        }

                        InputStatus::Handled
                    }

                    MotionAction::Move => {
                        for pointer in motion_event.pointers() {
                            receiver(create_touch_event(
                                motion_event,
                                &pointer,
                                TouchPhase::Move,
                                pixels_per_point,
                            ));
                        }

                        InputStatus::Handled
                    }

                    MotionAction::Cancel => {
                        for pointer in motion_event.pointers() {
                            receiver(create_touch_event(
                                motion_event,
                                &pointer,
                                TouchPhase::Cancel,
                                pixels_per_point,
                            ));
                        }

                        InputStatus::Handled
                    }

                    MotionAction::Outside => {
                        receiver(Event::PointerGone);
                        InputStatus::Handled
                    }

                    // Mouse over
                    MotionAction::HoverMove => {
                        for pointer in motion_event.pointers() {
                            if pointer.tool_type() == ToolType::Mouse {
                                receiver(Event::MouseMoved(
                                    vec2(
                                        pointer.axis_value(Axis::Hscroll),
                                        pointer.axis_value(Axis::Vscroll),
                                    ) / pixels_per_point,
                                ));
                            }
                        }

                        InputStatus::Handled
                    }

                    _ => InputStatus::Unhandled,
                }
            }

            // Some unknown event type.
            _ => InputStatus::Unhandled,
        }
    }
}

fn to_egui_key_event(key_event: &KeyEvent) -> Option<Event> {
    let physical_key = crate::keycodes::to_physical_key(key_event.key_code());

    if physical_key.is_none() {
        log::warn!("Unknown key code: {:?}", key_event.key_code());
        return None;
    }

    let pressed = match key_event.action() {
        KeyAction::Down => true,
        KeyAction::Up => false,
        KeyAction::Multiple => return None,
        _ => return None,
    };

    Some(Event::Key {
        key: physical_key.unwrap(),
        physical_key,
        pressed,
        repeat: false,
        modifiers: modifiers_from_meta_state(key_event.meta_state()),
    })
}

/// Derive keyboard modifiers from the meta state of an Android key event.
fn modifiers_from_meta_state(meta_state: MetaState) -> Modifiers {
    Modifiers {
        alt: meta_state.alt_on(),
        ctrl: meta_state.ctrl_on(),
        shift: meta_state.shift_on(),
        mac_cmd: false,
        command: meta_state.meta_on(),
    }
}

fn create_touch_event(
    motion_event: &MotionEvent,
    pointer: &Pointer,
    phase: TouchPhase,
    pixels_per_point: f32,
) -> Event {
    Event::Touch {
        device_id: TouchDeviceId(motion_event.device_id() as u64),
        id: TouchId(pointer.pointer_id() as u64),
        phase,
        pos: pos2(pointer.x(), pointer.y()) / pixels_per_point,
        force: Some(pointer.pressure()),
    }
}

fn create_click_event(
    motion_event: &MotionEvent,
    pointer: &Pointer,
    pressed: bool,
    pixels_per_point: f32,
) -> Event {
    Event::PointerButton {
        pos: pos2(pointer.x(), pointer.y()) / pixels_per_point,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    }
}
