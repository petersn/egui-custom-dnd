#![feature(let_chains)]

use std::{
  sync::atomic::AtomicU64, time::Instant,
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
enum ElementContents {
  Value(i32),
  Header(String),
}

#[derive(Clone)]
struct Element {
  id:           u64,
  list_y:       SlewPair,
  drag_y:       SlewPair,
  is_selected:  bool,
  contents:     ElementContents,
}

impl Element {
  fn from_value(contents: ElementContents) -> Self {
    Self {
      id: new_scratch_nonce(),
      list_y: SlewPair::default(),
      drag_y: SlewPair::default(),
      is_selected: false,
      contents,
    }
  }
}

const MAX_CLICK_RECORDS: usize = 4;

struct ClickRecord {
  id: u64,
  index: usize,
  time: Instant,
  ctrl_held: bool,
}

pub struct DndDemo {
  elements:            Vec<Element>,
  drag_state:          Option<DragState>,
  drop_region_hovered: bool,
  split_point:         usize,
  last_few_clicks:     Vec<ClickRecord>,
}

impl DndDemo {
  pub fn new() -> Self {
    Self {
      elements:     vec![
        Element::from_value(ElementContents::Header("Group A".to_string())),
        Element::from_value(ElementContents::Value(1)),
        Element::from_value(ElementContents::Value(2)),
        Element::from_value(ElementContents::Value(3)),
        Element::from_value(ElementContents::Header("Group B".to_string())),
        Element::from_value(ElementContents::Value(4)),
        Element::from_value(ElementContents::Value(5)),
        Element::from_value(ElementContents::Value(6)),
        Element::from_value(ElementContents::Value(7)),
      ],
      drag_state:   None,
      drop_region_hovered: false,
      split_point:  0,
      last_few_clicks: Vec::new(),
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
    for element in &mut self.elements {
      if element.is_selected || element.id == dragged_id {
        element.drag_y = SlewPair { current: drag_y, target: target_y };
        target_y += ITEM_HEIGHT;
      }
      drag_y += ITEM_HEIGHT;
    }
  }

  fn clear_drag_state(&mut self) {
    if !Self::have_active_drag(&self.drag_state) {
      self.drag_state = None;
      return;
    }
    println!("Clear drag!");
    // Complete the drag if the mouse is in the right region.
    let (before, after) = self.elements.split_at(self.split_point);
    let before = before.iter().filter(|e| !Self::is_part_of_drag(&self.drag_state, e));
    let middle = self.elements.iter().filter(|e| Self::is_part_of_drag(&self.drag_state, e));
    let after = after.iter().filter(|e| !Self::is_part_of_drag(&self.drag_state, e));
    self.elements = before.chain(middle).chain(after).cloned().collect();
    self.drag_state = None;
    // Clear selection.
    for element in &mut self.elements {
      element.is_selected = false;
    }
  }

  fn have_active_drag(drag_state: &Option<DragState>) -> bool {
    drag_state.map(|state| state.activated).unwrap_or(false)
  }

  fn is_part_of_drag(drag_state: &Option<DragState>, element: &Element) -> bool {
    if matches!(element.contents, ElementContents::Header(_)) {
      return false;
    }
    Self::have_active_drag(drag_state) && match &drag_state {
      Some(state) => state.dragged_id == element.id || element.is_selected,
      None => false,
    }
  }

  fn draw_element(
    force_show: bool,
    drag_state: &Option<DragState>,
    begin_drag_args: &mut Option<(egui::Pos2, u64)>,
    clear_drag_flag: &mut bool,
    last_few_clicks: &mut Vec<ClickRecord>,
    range_select: &mut Option<(u64, u64)>,
    clear_selection: &mut bool,
    element: &mut Element,
    element_index: usize,
    ui: &mut egui::Ui,
  ) {
    ui.horizontal(|ui| {
      if !matches!(element.contents, ElementContents::Header(_)) {
        let (rect, response) =
          ui.allocate_exact_size(egui::vec2(20.0, 20.0), egui::Sense::click_and_drag());
        if response.clicked_by(egui::PointerButton::Primary) {
          println!("Clicked!");
          // FIXME: Implement shift click to select a range.
          // Toggle selection.
          println!("Selected: {}", element.is_selected);
          element.is_selected ^= true;
          // Check if this is a ctrl click that selects an interval.
          let ctrl_held = ui.input(|inp| inp.modifiers.command_only());
          if let Some(record) = last_few_clicks.last() && ctrl_held {
            *range_select = Some((record.id, element.id));
          }
          last_few_clicks.push(ClickRecord {
            id: element.id,
            index: element_index,
            time: Instant::now(),
            ctrl_held,
          });
          if last_few_clicks.len() > 5 {
            last_few_clicks.remove(0);
          }
        }
        if response.drag_started_by(egui::PointerButton::Primary) {
          *begin_drag_args = Some((rect.left_top(), element.id));
        }
        if response.drag_released() {
          println!("Drag released!");
          *clear_drag_flag = true;
        }
        if !force_show && Self::is_part_of_drag(&drag_state, element) {
          return;
        }
        let color = match (element.is_selected, response.hovered() || force_show) {
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
      }
      // ui.label(format!("Element {}   {:.1} -> {:.1}", element.value, element.drag_y, element.target_y));
      match &element.contents {
        ElementContents::Value(value) => ui.label(format!("Element {}", value)),
        ElementContents::Header(value) => ui.label(format!("Header {}", value)),
      };
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

    // Split point starts at 1, because we never want to go above the top header.
    self.split_point = 1;
    let mut window_open = true;
    let mut begin_drag_args = None;
    let mut clear_drag_flag = false;
    let mut range_select = None;

    egui::Window::new("DndDemo").open(&mut window_open).resizable(true).show(egui_ctx, |ui| {
      ui.label(format!("Drag elements to reorder them. drag_state: {:?}", self.drag_state.is_some()));
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
          Self::draw_element(false, &self.drag_state, &mut begin_drag_args, &mut clear_drag_flag, &mut self.last_few_clicks, &mut range_select, element, element_index, ui);
        });
        // ui.put(rect, ElementRenderRequest {
        //   demo: self,
        //   index: element_index,
        //   hide_me: is_part_of_drag,
        // });
        if !is_part_of_drag && mouse_pos.y > rect.center().y {
          self.split_point = element_index + 1;
        }
      }
    });

    if let Some((element_left_top, dragged_id)) = begin_drag_args {
      self.begin_drag(mouse_pos, element_left_top, dragged_id);
    }

    if clear_drag_flag {
      self.clear_drag_state();
    }

    if have_active_drag {
      //println!("Split point: {}", self.split_point);
      let mut y = 0.0;
      for (element_index, element) in self.elements.iter_mut().enumerate() {
        if element_index == self.split_point {
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

    // }

    if let Some(drag_state) = &mut self.drag_state {
      // Shrink all gaps once we're at least 20 units away from the drag start pos.
      drag_state.activated |= (mouse_pos - drag_state.start_pos).length() > 5.0;
      if drag_state.activated {
        let grabbed_element =
          self.elements.iter_mut().find(|element| element.id == drag_state.dragged_id).unwrap();
        let grabbed_offset = mouse_pos + drag_state.offset;
        grabbed_element.drag_y.current = 0.0;

        for (element_index, element) in self.elements.iter_mut().enumerate() {
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
                Self::draw_element(true, &self.drag_state, &mut begin_drag_args, &mut clear_drag_flag, &mut self.last_few_clicks, &mut range_select, element, element_index, ui);
              });
          }
        }

        // We need to make sure that things animate properly.
        egui_ctx.request_repaint();
      }
    }

    if let Some((a, b)) = range_select {
      let get_index = |id| self.elements.iter().position(|e| e.id == id);
      if let (Some(a), Some(b)) = (get_index(a), get_index(b)) {
        let (a, b) = if a < b { (a, b) } else { (b, a) };
        for element in &mut self.elements[a..=b] {
          element.is_selected = true;
        }
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
