use winit;
use winit::WindowEvent;
use winit::KeyboardInput;
use frontend::input;
use frontend::input::Key;
use core::geometry::Position;

pub struct WinitEventMapper;

impl WinitEventMapper {
	pub fn new() -> Self {
		WinitEventMapper {}
	}
}

impl input::EventMapper<winit::WindowEvent> for WinitEventMapper {
	fn translate(&self, e: &winit::WindowEvent) -> Option<input::Event> {
		fn keymap(vk: winit::VirtualKeyCode) -> Option<input::Key> {
			macro_rules! winit_map (
				[$($gkey:ident -> $ekey:ident),*] => (
					match vk {
						$(winit::VirtualKeyCode::$gkey => Some(Key::$ekey)),
						*,
						_ => None,
					}
				)
			);
			winit_map![
				Key0 -> N0,
				Key1 -> N1,
				Key2 -> N2,
				Key3 -> N3,
				Key4 -> N4,
				Key5 -> N5,
				Key6 -> N6,
				Key7 -> N7,
				Key8 -> N8,
				Key9 -> N9,
				F1 -> F1,
				F2 -> F2,
				F3 -> F3,
				F4 -> F4,
				F5 -> F5,
				F6 -> F6,
				F7 -> F7,
				F8 -> F8,
				F9 -> F9,
				F10 -> F10,
				F11 -> F11,
				F12 -> F12,
				Home -> Home,
				Down -> Down,
				Up -> Up,
				Left -> Left,
				Right -> Right,
				PageUp -> PageUp,
				PageDown -> PageDown,
				LControl -> LCtrl,
				RControl -> RCtrl,
				LShift -> LShift,
				RShift -> RShift,
				LAlt -> LAlt,
				RAlt -> RAlt,
				LWin -> LSuper,
				RWin -> RSuper,
				A -> A,
				B -> B,
				C -> C,
				D -> D,
				E -> E,
				F -> F,
				G -> G,
				H -> H,
				I -> I,
				J -> J,
				K -> K,
				L -> L,
				M -> M,
				N -> N,
				O -> O,
				P -> P,
				Q -> Q,
				R -> R,
				S -> S,
				T -> T,
				U -> U,
				V -> V,
				W -> W,
				X -> X,
				Y -> Y,
				Z -> Z,
				Space -> Space,
				Escape -> Esc
			]
		}
		fn mousemap(button: winit::MouseButton) -> Option<input::Key> {
			match button {
				winit::MouseButton::Left => Some(input::Key::MouseLeft),
				winit::MouseButton::Right => Some(input::Key::MouseRight),
				winit::MouseButton::Middle => Some(input::Key::MouseMiddle),
				winit::MouseButton::Other(5) => Some(input::Key::MouseScrollUp),
				winit::MouseButton::Other(6) => Some(input::Key::MouseScrollDown),
				_ => None,
			}
		}
		fn state_map(element_state: winit::ElementState) -> input::State {
			match element_state {
				winit::ElementState::Pressed => input::State::Down,
				winit::ElementState::Released => input::State::Up,
			}
		}
		match e {
			&WindowEvent::KeyboardInput {
				input: KeyboardInput {
					state: element_state,
					virtual_keycode: vk,
					..
				},
				..
			} => {
				vk.and_then(|vk| keymap(vk)).and_then(|key| {
					Some(input::Event::Key(state_map(element_state), key))
				})
			}
			&WindowEvent::MouseInput {
				state: element_state,
				button,
				..
			} => mousemap(button).and_then(|key| Some(input::Event::Key(state_map(element_state), key))),
			&WindowEvent::CursorMoved { position: (x, y), .. } => Some(
				input::Event::Mouse(Position::new(x as f32, y as f32)),
			),
			_ => None,
		}
	}
}
