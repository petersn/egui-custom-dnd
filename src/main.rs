use std::{
  sync::atomic::AtomicU64,
};

use eframe::{egui, epaint::{pos2, vec2}};

const WIDTH: f32 = 300.0;
const ITEM_HEIGHT: f32 = 22.0;
const SLEW_RATE: f32 = 300.0;

fn new_scratch_nonce() -> u64 {
  static COUNTER: AtomicU64 = AtomicU64::new(0);
  COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

#[derive(Clone, Default)]
struct SlewPair {
  current: f32,
  target:  f32,
}

impl SlewPair {
  fn update(&mut self, dt: f32) {
    let diff = self.target - self.current;
    let delta = (dt * SLEW_RATE * diff.signum()).clamp(-diff.abs(), diff.abs());
    self.current += delta;
  }
}

#[derive(Clone, Copy)]
struct DragState {
  activated:  bool,
  start_pos:  egui::Pos2,
  offset:     egui::Vec2,
  dragged_id: u64,
}

#[derive(Clone)]
struct Element {
  id:           u64,
  value:        i32,
  list_y:       SlewPair,
  drag_y:       SlewPair,
  is_selected:  bool,
}

impl Element {
  fn from_value(value: i32) -> Self {
    Self {
      id: new_scratch_nonce(),
      value,
      list_y: SlewPair::default(),
      drag_y: SlewPair::default(),
      is_selected: false,
    }
  }
}

// struct ElementRenderRequest<'a> {
//   demo:    &'a mut DndDemo,
//   index:   usize,
//   hide_me: bool,
// }

// impl<'a> egui::Widget for ElementRenderRequest<'a> {
//   fn ui(self, ui: &mut egui::Ui) -> egui::Response {
//     let ElementRenderRequest { demo, index, hide_me } = self;
    
//     .response
//   }
// }

pub struct DndDemo {
  elements:            Vec<Element>,
  drag_state:          Option<DragState>,
  drop_region_hovered: bool,
}

impl DndDemo {
  pub fn new() -> Self {
    Self {
      elements:     (1..=5).map(Element::from_value).collect(),
      drag_state:   None,
      drop_region_hovered: false,
    }
  }

  fn begin_drag(
    &mut self,
    mouse_pos: egui::Pos2,
    element_left_top: egui::Pos2,
    dragged_id: u64,
  ) {
    self.drag_state = Some(DragState {
      activated: false,
      start_pos: mouse_pos,
      offset: element_left_top - mouse_pos,
      dragged_id,
    });
    // Setup the y offsets.
    let count_before_drag_element = self.elements
      .iter()
      .take_while(|element| element.id != dragged_id)
      .count();
    let selected_count_before_drag_element = self.elements
      .iter()
      .take_while(|element| element.id != dragged_id)
      .filter(|element| element.is_selected)
      .count();
    let mut drag_y = -(count_before_drag_element as f32) * ITEM_HEIGHT;
    let mut target_y = -(selected_count_before_drag_element as f32) * ITEM_HEIGHT;
    println!("Drag y: {} -- Target y: {}", drag_y, target_y);
    for element in &mut self.elements {
      if element.is_selected || element.id == dragged_id {
        element.drag_y = SlewPair { current: drag_y, target: target_y };
        target_y += ITEM_HEIGHT;
      }
      drag_y += ITEM_HEIGHT;
    }
  }

  fn clear_drag_state(&mut self) {
    self.drag_state = None;
  }

  fn have_active_drag(drag_state: &Option<DragState>) -> bool {
    drag_state.map(|state| state.activated).unwrap_or(false)
  }

  fn is_part_of_drag(drag_state: &Option<DragState>, element: &Element) -> bool {
    Self::have_active_drag(drag_state) && match &drag_state {
      Some(state) => state.dragged_id == element.id || element.is_selected,
      None => false,
    }
  }

  fn draw_element(
    force_show: bool,
    drag_state: &Option<DragState>,
    begin_drag_args: &mut Option<(egui::Pos2, u64)>,
    element: &mut Element,
    ui: &mut egui::Ui,
  ) {
    ui.horizontal(|ui| {
      let (rect, response) =
        ui.allocate_exact_size(egui::vec2(20.0, 20.0), egui::Sense::click_and_drag());
      if response.clicked_by(egui::PointerButton::Primary) {
        println!("Clicked!");
        // FIXME: Implement shift click to select a range.
        // Toggle selection.
        element.is_selected ^= true;
        println!("Selected: {}", element.is_selected);
      }
      if response.drag_started_by(egui::PointerButton::Primary) {
        *begin_drag_args = Some((rect.left_top(), element.id));
      }
      if !force_show && Self::is_part_of_drag(&drag_state, element) {
        return;
      }
      let color = match (element.is_selected, response.hovered()) {
        (true, _) => egui::Color32::from_rgb(100, 100, 250),
        (false, true) => egui::Color32::from_rgb(100, 100, 175),
        (false, false) => egui::Color32::from_rgb(100, 100, 100),
      };
      let dim_color = egui::Color32::from_rgb(color.r() - 25, color.g() - 25, color.b() - 25);
      ui.painter().rect_filled(rect, 3.0, color);
      for i in 0..3 {
        let rect = egui::Rect::from_min_size(
          rect.left_top() + egui::vec2(2.0, 2.0 + 6.0 * i as f32),
          egui::vec2(16.0, 4.0),
        );
        ui.painter().rect_filled(rect, 2.0, dim_color);
      }
      // ui.label(format!("Element {}   {:.1} -> {:.1}", element.value, element.drag_y, element.target_y));
      ui.label(format!("Element {}", element.value));
      if ui.button("Delete").clicked() {
        //self.value = 0;
      }
    });
  }

  pub fn demo(&mut self, egui_ctx: &egui::Context) {
    let (dt, mouse_pos) = egui_ctx.input(|inp| (inp.unstable_dt, inp.pointer.interact_pos().unwrap_or_default()));
    if !egui_ctx.memory(|mem| mem.is_anything_being_dragged()) {
      self.clear_drag_state();
    }
    let have_active_drag = Self::have_active_drag(&self.drag_state);

    // egui::containers::popup::show_tooltip_at(egui_ctx, "asdf".into(), Some(egui::pos2(50.0, 50.0)), |ui| {
    //   ui.label("Drag elements to reorder them.");
    // });

    // let (currently_dragging, drag_element) = match self.drag_state.borrow().as_ref() {
    //   Some(state) => (state.drag_shrink_started, state.drag_element),
    //   None => (false, u64::MAX),
    // };

    if !have_active_drag {
      // Layout the elements in order.
      let mut y = 0.0;
      for element in &mut self.elements {
        element.list_y = SlewPair { current: y, target: y };
        y += ITEM_HEIGHT;
      }
    }

    let drag_count = self.elements.iter().filter(|element| Self::is_part_of_drag(&self.drag_state, element)).count();

    let mut split_point = 0;
    let mut window_open = true;
    let mut begin_drag_args = None;

    egui::Window::new("DndDemo").open(&mut window_open).resizable(true).show(egui_ctx, |ui| {
      ui.label("Drag elements to reorder them.");
      let spot = ui.next_widget_position();
      //let spot = egui::Pos2::new(100.0, 100.0);
      //println!("Spot: {:?}", spot);
      let box_size = vec2(WIDTH, ITEM_HEIGHT * self.elements.len() as f32);
      let (full_box_rect, _) = ui.allocate_exact_size(box_size, egui::Sense::hover());
      self.drop_region_hovered = full_box_rect.contains(mouse_pos);
      // if self.drop_region_hovered {
      //   ui.painter().rect_filled(full_box_rect, 0.0, egui::Color32::from_rgb(100, 100, 150));
      // }

      // Place the elements inside the window.
      for (element_index, element) in self.elements.iter_mut().enumerate() {
        // let hide_me = currently_dragging && (element.is_selected || element.id == drag_element);
        // if hide_me {
        //   drag_count += 1;
        // }
        let mut rect = egui::Rect::NOTHING;
        rect.set_left(spot.x);
        rect.set_right(spot.x + 300.0);
        rect.set_top(spot.y + element.list_y.current);
        rect.set_bottom(spot.y + element.list_y.current + ITEM_HEIGHT);
        //println!("Rect: {:?}", rect);
        let is_part_of_drag = Self::is_part_of_drag(&self.drag_state, element);
        ui.allocate_ui_at_rect(rect, |ui| {
          Self::draw_element(false, &self.drag_state, &mut begin_drag_args, element, ui);
        });
        // ui.put(rect, ElementRenderRequest {
        //   demo: self,
        //   index: element_index,
        //   hide_me: is_part_of_drag,
        // });
        if !is_part_of_drag && mouse_pos.y > rect.center().y {
          split_point = element_index + 1;
        }
      }
    });

    if let Some((element_left_top, dragged_id)) = begin_drag_args {
      println!("Begin drag!");
      self.begin_drag(mouse_pos, element_left_top, dragged_id);
    }

    if have_active_drag {
      //println!("Split point: {}", split_point);
      let mut y = 0.0;
      for (element_index, element) in self.elements.iter_mut().enumerate() {
        if element_index == split_point {
          y += ITEM_HEIGHT * drag_count as f32;
        }
        if !Self::is_part_of_drag(&self.drag_state, element) {
          element.list_y.target = y;
          y += ITEM_HEIGHT;
        }
      }
    }
    // if drag_state_was_some && !is_anything_being_dragged && self.inhibit_drop == 0 {
    //   println!("Complete drag!");
    //   // Complete the drag if the mouse is in the right region.
    //   if drop_region_hovered {
    //     println!("Drop region hovered: {}", split_point);
    //     let mut new_elements = Vec::new();
    //     for (index, element) in self.elements.borrow().iter().enumerate() {
    //       if index == split_point {
    //         println!("Inserting drag elements");
    //         for element in self.elements.borrow().iter() {
    //           if element.is_selected {
    //             println!("  Inserting element {}", element.value);
    //             new_elements.push(element.clone());
    //           }
    //         }
    //       }
    //       if !element.hide_me {
    //         println!("Inserting element {}", element.value);
    //         new_elements.push(element.clone());
    //       }
    //     }
    //     *self.elements.borrow_mut() = new_elements;
    //   }
    // }

    if let Some(drag_state) = &mut self.drag_state {
      // Shrink all gaps once we're at least 20 units away from the drag start pos.
      drag_state.activated |= (mouse_pos - drag_state.start_pos).length() > 5.0;
      if drag_state.activated {
        let grabbed_element =
          self.elements.iter_mut().find(|element| element.id == drag_state.dragged_id).unwrap();
        let grabbed_offset = mouse_pos + drag_state.offset;
        grabbed_element.drag_y.current = 0.0;

        for element in &mut self.elements {
          let y = match Self::is_part_of_drag(&self.drag_state, element) {
            true => &mut element.drag_y,
            false => &mut element.list_y,
          };
          y.update(dt);
          if Self::is_part_of_drag(&self.drag_state, element) {
            egui::Area::new(format!("elem_{}", element.id))
              .interactable(false)
              .fixed_pos(pos2(grabbed_offset.x, grabbed_offset.y + element.drag_y.current))
              .order(egui::Order::Foreground)
              .show(egui_ctx, |ui| {
                Self::draw_element(true, &self.drag_state, &mut begin_drag_args, element, ui);
              });
          }
        }

        // We need to make sure that things animate properly.
        egui_ctx.request_repaint();
      }
    }
  }
}

struct App {
  demo: DndDemo,
}

impl App {
  fn new(_cc: &eframe::CreationContext) -> Self {
    Self {
      demo: DndDemo::new(),
    }
  }
}

impl eframe::App for App {
  fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
    self.demo.demo(ctx);
  }
}

fn main() -> Result<(), eframe::Error> {
  eframe::run_native(
    "Template",
    eframe::NativeOptions::default(),
    Box::new(|cc| Box::new(App::new(cc))),
  )
}
