use std::{
  cell::{Cell, RefCell},
  collections::HashSet,
  rc::{Rc, Weak},
  sync::atomic::AtomicU64,
};

use eframe::{egui, epaint::{pos2, vec2}};

const SLEW_RATE: f32 = 300.0;

fn new_scratch_nonce() -> u64 {
  static COUNTER: AtomicU64 = AtomicU64::new(0);
  COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

#[derive(Clone, Copy)]
struct DragState {
  drag_shrink_started: bool,
  drag_start_pos:      egui::Pos2,
  drag_offset:         egui::Vec2,
  drag_element:        u64,
}

#[derive(Clone)]
struct Element {
  demo:         Weak<DndDemo>,
  id:           u64,
  list_y:       Cell<f32>,
  drag_y:       Cell<f32>,
  target_y:     Cell<f32>,
  value:        i32,
  is_selected:  Cell<bool>,
  hide_me:      Cell<bool>,
}

impl Element {
  fn from_value(demo: Weak<DndDemo>, value: i32) -> Self {
    Self {
      demo,
      id: new_scratch_nonce(),
      list_y: Cell::new(0.0),
      drag_y: Cell::new(0.0),
      target_y: Cell::new(0.0),
      value,
      is_selected: Cell::new(false),
      hide_me: Cell::new(false),
    }
  }

  fn draw(&self, ui: &mut egui::Ui) -> egui::Response {
    let mouse_pos = ui.ctx().input(|i| i.pointer.interact_pos()).unwrap_or_default();
    ui.horizontal(|ui| {
      let (rect, response) =
        ui.allocate_exact_size(egui::vec2(20.0, 20.0), egui::Sense::click_and_drag());
      if response.clicked_by(egui::PointerButton::Primary) {
        println!("Clicked!");
        // FIXME: Implement shift click to select a range.
        // Toggle selection.
        self.is_selected.set(!self.is_selected.get());
        println!("Selected: {}", self.is_selected.get());
      }
      if response.drag_started_by(egui::PointerButton::Primary) {
        self.demo.upgrade().unwrap().begin_drag(mouse_pos, rect.left_top(), self.id);
      }

      if self.hide_me.get() {
        return;
      }

      let color = match (self.is_selected.get(), response.hovered()) {
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
      ui.label(format!("Element {}   {:.1} -> {:.1}", self.value, self.drag_y.get(), self.target_y.get()));
      if ui.button("Delete").clicked() {
        //self.value = 0;
      }
    })
    .response
  }
}

impl<'a> egui::Widget for &'a Element {
  fn ui(self, ui: &mut egui::Ui) -> egui::Response {
    self.draw(ui)
  }
}

pub struct DndDemo {
  elements:   RefCell<Vec<Element>>,
  drag_state: RefCell<Option<DragState>>,
  inhibit_drop: Cell<u8>,
}

impl DndDemo {
  pub fn new() -> Rc<Self> {
    let this = Rc::new(Self {
      elements:   RefCell::new(Vec::new()),
      drag_state: RefCell::new(None),
      inhibit_drop: Cell::new(0),
    });
    {
      let mut elements = this.elements.borrow_mut();
      for i in 1..=5 {
        elements.push(Element::from_value(Rc::downgrade(&this), i));
      }
    }
    this
  }

  fn begin_drag(
    self: Rc<Self>,
    mouse_pos: egui::Pos2,
    dragged_element_rect: egui::Pos2,
    drag_element: u64,
  ) {
    let mut drag_state = self.drag_state.borrow_mut();
    *drag_state = Some(DragState {
      drag_shrink_started: false,
      drag_start_pos: mouse_pos,
      drag_offset: dragged_element_rect - mouse_pos,
      drag_element,
    });
    // Setup the y offsets.
    let elements = self.elements.borrow();
    let count_before_drag_element = elements
      .iter()
      .take_while(|element| element.id != drag_element)
      .count();
    let selected_count_before_drag_element = elements
      .iter()
      .take_while(|element| element.id != drag_element)
      .filter(|element| element.is_selected.get())
      .count();
    let mut drag_y = -(count_before_drag_element as f32) * 22.0;
    let mut target_y = -(selected_count_before_drag_element as f32) * 22.0;
    for element in elements.iter() {
      if element.is_selected.get() || element.id == drag_element {
        element.target_y.set(target_y);
        element.drag_y.set(drag_y);
        target_y += 22.0;
      }
      drag_y += 22.0;
    }
    self.inhibit_drop.set(3);
  }

  pub fn demo(&self, egui_ctx: &egui::Context) {
    let drag_state_was_some = self.drag_state.borrow().is_some();
    let is_anything_being_dragged = egui_ctx.memory(|mem| mem.is_anything_being_dragged());
    if !is_anything_being_dragged {
      *self.drag_state.borrow_mut() = None;
    }
    //let drag_state: Option<DragState> = *self.drag_state.lock().unwrap();
    let mouse_pos = egui_ctx.input(|i| i.pointer.interact_pos()).unwrap_or_default();

    // egui::containers::popup::show_tooltip_at(egui_ctx, "asdf".into(), Some(egui::pos2(50.0, 50.0)), |ui| {
    //   ui.label("Drag elements to reorder them.");
    // });

    let (currently_dragging, drag_element) = match self.drag_state.borrow().as_ref() {
      Some(state) => (state.drag_shrink_started, state.drag_element),
      None => (false, u64::MAX),
    };

    if self.drag_state.borrow().is_none() {
      // Layout the elements in order.
      let mut y = 0.0;
      for element in self.elements.borrow().iter() {
        element.list_y.set(y);
        element.target_y.set(y);
        y += 22.0;
      }
    }

    let mut split_point = 0;
    let mut window_open = true;
    let mut drag_count = 0;
    let mut drop_region_hovered = false;
    egui::Window::new("DndDemo").open(&mut window_open).resizable(true).show(egui_ctx, |ui| {
      ui.label("Drag elements to reorder them.");
      let spot = ui.next_widget_position();
      //let spot = egui::Pos2::new(100.0, 100.0);
      //println!("Spot: {:?}", spot);
      let box_size = vec2(300.0, 22.0 * self.elements.borrow().len() as f32);
      let (_, full_box_response) = ui.allocate_exact_size(box_size, egui::Sense::click_and_drag());
      drop_region_hovered = full_box_response.hovered();
      for (element_index, element) in self.elements.borrow().iter().enumerate() {
        let hide_me = currently_dragging && (element.is_selected.get() || element.id == drag_element);
        if hide_me {
          drag_count += 1;
        }
        element.hide_me.set(hide_me);
        let mut rect = egui::Rect::NOTHING;
        rect.set_left(spot.x);
        rect.set_right(spot.x + 300.0);
        rect.set_top(spot.y + element.list_y.get());
        rect.set_bottom(spot.y + element.list_y.get() + 22.0);
        //println!("Rect: {:?}", rect);
        ui.put(rect, element);
        if !hide_me && currently_dragging && mouse_pos.y > rect.center().y {
          split_point = element_index + 1;
        }
      }
    });
    if currently_dragging {
      //println!("Split point: {}", split_point);
      let mut y = 0.0;
      let mut elements = self.elements.borrow_mut();
      for (element_index, element) in elements.iter_mut().enumerate() {
        if element_index == split_point {
          y += 22.0 * drag_count as f32;
        }
        if !element.hide_me.get() {
          element.target_y.set(y);
          y += 22.0;
        }
      }
    }
    if drag_state_was_some && !is_anything_being_dragged && self.inhibit_drop.get() == 0 {
      println!("Complete drag!");
      // Complete the drag if the mouse is in the right region.
      if drop_region_hovered {
        println!("Drop region hovered: {}", split_point);
        let mut new_elements = Vec::new();
        for (index, element) in self.elements.borrow().iter().enumerate() {
          if index == split_point {
            println!("Inserting drag elements");
            for element in self.elements.borrow().iter() {
              if element.is_selected.get() {
                println!("  Inserting element {}", element.value);
                new_elements.push(element.clone());
              }
            }
          }
          if !element.hide_me.get() {
            println!("Inserting element {}", element.value);
            new_elements.push(element.clone());
          }
        }
        *self.elements.borrow_mut() = new_elements;
      }
    }

    if let Some(drag_state) = &mut *self.drag_state.borrow_mut() {
      // Shrink all gaps once we're at least 20 units away from the drag start pos.
      drag_state.drag_shrink_started |= (mouse_pos - drag_state.drag_start_pos).length() > 5.0;
      if drag_state.drag_shrink_started {
        let elements = self.elements.borrow();
        let grabbed_element =
          elements.iter().find(|element| element.id == drag_state.drag_element).unwrap();
        let grabbed_offset = mouse_pos + drag_state.drag_offset;
        grabbed_element.drag_y.set(0.0);

        let dt = egui_ctx.input(|inp| inp.unstable_dt);
        for element in elements.iter() {
          let y = match element.hide_me.get() {
            true => &element.drag_y,
            false => &element.list_y,
          };
          let diff = element.target_y.get() - y.get();
          let delta = (dt * SLEW_RATE * diff.signum()).clamp(-diff.abs(), diff.abs());
          y.set(y.get() + delta);
          if element.is_selected.get() || element.id == drag_state.drag_element {
            egui::Area::new(format!("elem_{}", element.id))
              .interactable(false)
              .fixed_pos(pos2(grabbed_offset.x, grabbed_offset.y + element.drag_y.get()))
              .order(egui::Order::Foreground)
              .show(egui_ctx, |ui| {
                element.hide_me.set(false);
                element.draw(ui);
              });
          }
        }

        // We need to make sure that things animate properly.
        egui_ctx.request_repaint();
      }
    }

    self.inhibit_drop.set(self.inhibit_drop.get().saturating_sub(1));
  }
}

fn main() -> Result<(), eframe::Error> {
  eframe::run_native(
    "Template",
    eframe::NativeOptions::default(),
    Box::new(|cc| Box::new(App::new(cc))),
  )
}

struct App {
  demo: Rc<DndDemo>,
}

impl App {
  fn new(_cc: &eframe::CreationContext) -> Self {
    Self {
      demo: DndDemo::new(),
    }
  }
}

impl eframe::App for App {
  fn update(&mut self, _ctx: &egui::Context, _frame: &mut eframe::Frame) {
    self.demo.demo(_ctx);
  }
}
