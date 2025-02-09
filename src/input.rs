use android_activity::{
    input::{
        Axis, InputEvent, KeyAction, KeyEvent, KeyMapChar, Keycode, MetaState, MotionAction,
        MotionEvent, Pointer, ToolType,
    },
    AndroidApp, InputStatus,
};
use egui::{
    pos2, vec2, Event, Modifiers, MouseWheelUnit, PointerButton, Pos2, TouchDeviceId, TouchId,
    TouchPhase,
};

/// Stateful object that processes input events from Android, and translates
/// them into egui input events.
pub(crate) struct InputHandler {
    app: AndroidApp,
    combining_accent: Option<char>,
}

impl InputHandler {
    pub fn new(app: AndroidApp) -> Self {
        Self {
            app,
            combining_accent: None,
        }
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
        match android_event {
            InputEvent::KeyEvent(key_event) => self.process_key_event(key_event, receiver),

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

                    MotionAction::Down | MotionAction::PointerDown => {
                        for pointer in motion_event.pointers() {
                            receiver(create_touch_event(
                                motion_event,
                                &pointer,
                                TouchPhase::Start,
                                pixels_per_point,
                            ));
                        }

                        if motion_event.pointer_count() == 1 {
                            receiver(create_click_event(
                                motion_event,
                                &motion_event.pointers().next().unwrap(),
                                true,
                                pixels_per_point,
                            ));
                        }

                        InputStatus::Handled
                    }

                    MotionAction::Up | MotionAction::PointerUp => {
                        for pointer in motion_event.pointers() {
                            receiver(create_touch_event(
                                motion_event,
                                &pointer,
                                TouchPhase::End,
                                pixels_per_point,
                            ));
                        }

                        if motion_event.pointer_count() == 1 {
                            receiver(create_click_event(
                                motion_event,
                                &motion_event.pointers().next().unwrap(),
                                false,
                                pixels_per_point,
                            ));
                        }

                        if motion_event.pointer_count() <= 1 {
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

                        if motion_event.pointer_count() == 1 {
                            let pointer = motion_event.pointers().next().unwrap();
                            receiver(Event::PointerMoved(pointer_pos(&pointer, pixels_per_point)));
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

                    e => {
                        log::warn!("unknown motion event: {e:?}");
                        InputStatus::Unhandled
                    }
                }
            }

            // Some unknown event type.
            InputEvent::TextEvent(text_event) => {
                log::warn!("unhandled text event: {text_event:?}");
                InputStatus::Unhandled
            }

            unknown => {
                log::warn!("unhandled input event: {unknown:?}");
                InputStatus::Unhandled
            }
        }
    }

    fn process_key_event(
        &mut self,
        key_event: &KeyEvent,
        mut receiver: impl FnMut(Event),
    ) -> InputStatus {
        let device_id = key_event.device_id();

        let key_map = match self.app.device_key_character_map(device_id) {
            Ok(key_map) => key_map,
            Err(err) => {
                log::warn!("failed to look up `KeyCharacterMap` for device {device_id}: {err:?}");
                return InputStatus::Unhandled;
            }
        };

        let cma = match key_map.get(key_event.key_code(), key_event.meta_state()) {
            Ok(c) => c,
            Err(err) => {
                log::warn!("KeyEvent: Failed to get key map character: {err:?}");
                KeyMapChar::None
            }
        };

        match cma {
            KeyMapChar::Unicode(unicode) => {
                if key_event.action() == KeyAction::Down {
                    if let Some(combining_accent) = self.combining_accent.take() {
                        if let Some(c) = key_map
                            .get_dead_char(combining_accent, unicode)
                            .inspect_err(|e| {
                                log::warn!(
                                    "KeyEvent: Failed to combine 'dead key' accent '{combining_accent}' with \
                                    '{unicode}': {e:?}"
                                )
                            })
                            .ok()
                            .flatten()
                        {
                            receiver(Event::Text(c.into()));
                            InputStatus::Handled
                        } else {
                            InputStatus::Unhandled
                        }
                    } else {
                        receiver(Event::Text(unicode.into()));
                        InputStatus::Handled
                    }
                } else {
                    InputStatus::Handled
                }
            }
            KeyMapChar::CombiningAccent(combining_accent) => {
                self.combining_accent = Some(combining_accent);
                InputStatus::Handled
            }
            KeyMapChar::None => match key_event.key_code() {
                Keycode::Copy => {
                    receiver(Event::Copy);
                    InputStatus::Handled
                }
                Keycode::Cut => {
                    receiver(Event::Cut);
                    InputStatus::Handled
                }
                keycode => {
                    if let Some(key) = crate::keycodes::to_physical_key(keycode) {
                        receiver(Event::Key {
                            key,
                            physical_key: None,
                            pressed: key_event.action() == KeyAction::Down,
                            repeat: key_event.repeat_count() > 0,
                            modifiers: modifiers_from_meta_state(key_event.meta_state()),
                        });
                        InputStatus::Handled
                    } else {
                        log::warn!("Unknown key code: {:?}", key_event.key_code());
                        InputStatus::Unhandled
                    }
                }
            },
        }
    }
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
        pos: pointer_pos(pointer, pixels_per_point),
        force: Some(pointer.pressure()),
    }
}

fn create_click_event(
    _motion_event: &MotionEvent,
    pointer: &Pointer,
    pressed: bool,
    pixels_per_point: f32,
) -> Event {
    Event::PointerButton {
        pos: pointer_pos(pointer, pixels_per_point),
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    }
}

fn pointer_pos(pointer: &Pointer, pixels_per_point: f32) -> Pos2 {
    pos2(pointer.x(), pointer.y()) / pixels_per_point
}
