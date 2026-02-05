use crate::display::{InputEvent, MouseButtonKind, PixelBuffer};
use crate::regions::{Circle, Point, Polygon, Region, Scene, Shape};

/// State machine for calibration mode
#[derive(Debug, Clone)]
enum State {
    /// Nothing selected, not drawing
    Idle,
    /// Drawing a new polygon
    Drawing { vertices: Vec<Point> },
    /// Drawing a circle (shift+click to start, drag to set radius)
    DrawingCircle { center: Point },
    /// A region is selected for editing
    Selected { region_index: usize },
    /// Dragging a vertex of the selected region (polygon only)
    DraggingVertex {
        region_index: usize,
        vertex_index: usize,
    },
    /// Dragging the center of a circle region
    DraggingCircleCenter { region_index: usize },
    /// Resizing a circle region (dragging edge)
    ResizingCircle { region_index: usize },
}

/// Calibration mode for defining scene regions with the mouse
pub struct CalibrationMode {
    state: State,
    scene: Scene,
    mouse_pos: (i32, i32),
    mouse_down: bool,
    shift_held: bool,
    next_region_name: String,
    snap_distance: f32,
    vertex_handle_size: f32,
}

impl CalibrationMode {
    pub fn new(scene: Scene) -> Self {
        // Migrate any legacy regions on load
        let mut scene = scene;
        for region in &mut scene.regions {
            region.migrate_legacy();
        }
        Self {
            state: State::Idle,
            scene,
            mouse_pos: (0, 0),
            mouse_down: false,
            shift_held: false,
            next_region_name: "region_1".to_string(),
            snap_distance: 12.0,
            vertex_handle_size: 8.0,
        }
    }

    pub fn scene(&self) -> &Scene {
        &self.scene
    }

    pub fn scene_mut(&mut self) -> &mut Scene {
        &mut self.scene
    }

    pub fn handle_event(&mut self, event: &InputEvent) {
        match event {
            InputEvent::MouseMove { x, y } => {
                self.mouse_pos = (*x, *y);
                self.on_mouse_move();
            }
            InputEvent::MouseDown { x, y, button } => {
                self.mouse_pos = (*x, *y);
                if *button == MouseButtonKind::Left {
                    self.mouse_down = true;
                    self.on_left_click();
                } else if *button == MouseButtonKind::Right {
                    self.on_right_click();
                }
            }
            InputEvent::MouseUp { button, .. } => {
                if *button == MouseButtonKind::Left {
                    self.mouse_down = false;
                    self.on_mouse_up();
                }
            }
            InputEvent::KeyDown(key) => {
                use sdl2::keyboard::Keycode;
                if *key == Keycode::LShift || *key == Keycode::RShift {
                    self.shift_held = true;
                }
            }
            InputEvent::KeyUp(key) => {
                use sdl2::keyboard::Keycode;
                if *key == Keycode::LShift || *key == Keycode::RShift {
                    self.shift_held = false;
                }
            }
            _ => {}
        }
    }

    fn on_mouse_move(&mut self) {
        match self.state {
            State::DraggingVertex {
                region_index,
                vertex_index,
            } => {
                if self.mouse_down {
                    if let Some(region) = self.scene.regions.get_mut(region_index) {
                        if let Some(poly) = region.polygon_mut() {
                            if let Some(vertex) = poly.vertices.get_mut(vertex_index) {
                                vertex.x = self.mouse_pos.0 as f32;
                                vertex.y = self.mouse_pos.1 as f32;
                            }
                        }
                    }
                }
            }
            State::DraggingCircleCenter { region_index } => {
                if self.mouse_down {
                    if let Some(region) = self.scene.regions.get_mut(region_index) {
                        if let Some(circle) = region.circle_mut() {
                            circle.center.x = self.mouse_pos.0 as f32;
                            circle.center.y = self.mouse_pos.1 as f32;
                        }
                    }
                }
            }
            State::ResizingCircle { region_index } => {
                if self.mouse_down {
                    if let Some(region) = self.scene.regions.get_mut(region_index) {
                        if let Some(circle) = region.circle_mut() {
                            let dx = self.mouse_pos.0 as f32 - circle.center.x;
                            let dy = self.mouse_pos.1 as f32 - circle.center.y;
                            circle.radius = (dx * dx + dy * dy).sqrt().max(10.0);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn on_left_click(&mut self) {
        let click = Point::new(self.mouse_pos.0 as f32, self.mouse_pos.1 as f32);

        match &self.state {
            State::Idle => {
                // Check if clicking on an existing region
                if let Some(idx) = self.find_region_at(click.x, click.y) {
                    let region = &self.scene.regions[idx];
                    if region.is_circle() {
                        // For circles, check if clicking near center or edge
                        if let Some(Shape::Circle(c)) = &region.shape {
                            let dist_to_center = click.distance_to(&c.center);
                            if dist_to_center < self.vertex_handle_size * 2.0 {
                                // Clicking near center - drag the circle
                                self.state = State::DraggingCircleCenter { region_index: idx };
                            } else if (dist_to_center - c.radius).abs() < self.vertex_handle_size {
                                // Clicking near edge - resize
                                self.state = State::ResizingCircle { region_index: idx };
                            } else {
                                self.state = State::Selected { region_index: idx };
                            }
                        }
                    } else {
                        // Polygon region
                        if let Some(vidx) = self.find_vertex_at(idx, click.x, click.y) {
                            self.state = State::DraggingVertex {
                                region_index: idx,
                                vertex_index: vidx,
                            };
                        } else {
                            self.state = State::Selected { region_index: idx };
                        }
                    }
                } else if self.shift_held {
                    // Shift+click on empty space - start drawing circle
                    self.state = State::DrawingCircle { center: click };
                } else {
                    // Start drawing new polygon
                    self.state = State::Drawing {
                        vertices: vec![click],
                    };
                }
            }

            State::Drawing { vertices } => {
                // Check if clicking near first vertex to close
                if vertices.len() >= 3 {
                    let first = &vertices[0];
                    if click.distance_to(first) < self.snap_distance {
                        // Close the polygon and create region
                        let polygon = Polygon::from_vertices(vertices.clone());
                        let region = Region::new(self.next_region_name.clone(), polygon);
                        let new_idx = self.scene.regions.len();
                        self.scene.add_region(region);
                        self.auto_increment_name();
                        self.state = State::Selected {
                            region_index: new_idx,
                        };
                        return;
                    }
                }
                // Add vertex
                let mut new_verts = vertices.clone();
                new_verts.push(click);
                self.state = State::Drawing {
                    vertices: new_verts,
                };
            }

            State::DrawingCircle { .. } => {
                // Circle is finalized on mouse up, not click
            }

            State::Selected { region_index } => {
                let idx = *region_index;
                let region = &self.scene.regions[idx];

                if region.is_circle() {
                    if let Some(Shape::Circle(c)) = &region.shape {
                        let dist_to_center = click.distance_to(&c.center);
                        if dist_to_center < self.vertex_handle_size * 2.0 {
                            self.state = State::DraggingCircleCenter { region_index: idx };
                            return;
                        } else if (dist_to_center - c.radius).abs() < self.vertex_handle_size {
                            self.state = State::ResizingCircle { region_index: idx };
                            return;
                        }
                    }
                } else if let Some(vidx) = self.find_vertex_at(idx, click.x, click.y) {
                    self.state = State::DraggingVertex {
                        region_index: idx,
                        vertex_index: vidx,
                    };
                    return;
                }

                // Check if clicking on a different region
                if let Some(new_idx) = self.find_region_at(click.x, click.y) {
                    if new_idx != idx {
                        self.state = State::Selected {
                            region_index: new_idx,
                        };
                    }
                } else if self.shift_held {
                    self.state = State::DrawingCircle { center: click };
                } else {
                    self.state = State::Drawing {
                        vertices: vec![click],
                    };
                }
            }

            State::DraggingVertex { .. }
            | State::DraggingCircleCenter { .. }
            | State::ResizingCircle { .. } => {
                // Shouldn't happen - drag starts on mouse down
            }
        }
    }

    fn on_right_click(&mut self) {
        // Cancel/deselect - return to idle from any active state
        match &self.state {
            State::Idle => {}
            _ => {
                self.state = State::Idle;
            }
        }
    }

    fn on_mouse_up(&mut self) {
        match self.state {
            State::DraggingVertex { region_index, .. }
            | State::DraggingCircleCenter { region_index }
            | State::ResizingCircle { region_index } => {
                self.state = State::Selected { region_index };
            }
            State::DrawingCircle { center } => {
                // Finalize circle with radius from center to current mouse position
                let dx = self.mouse_pos.0 as f32 - center.x;
                let dy = self.mouse_pos.1 as f32 - center.y;
                let radius = (dx * dx + dy * dy).sqrt();

                if radius >= 10.0 {
                    let circle = Circle::new(center, radius);
                    let region = Region::new_circle(self.next_region_name.clone(), circle);
                    let new_idx = self.scene.regions.len();
                    self.scene.add_region(region);
                    self.auto_increment_name();
                    self.state = State::Selected {
                        region_index: new_idx,
                    };
                } else {
                    // Too small, cancel
                    self.state = State::Idle;
                }
            }
            _ => {}
        }
    }

    /// Delete the currently selected region
    pub fn delete_selected(&mut self) {
        if let State::Selected { region_index }
        | State::DraggingVertex { region_index, .. }
        | State::DraggingCircleCenter { region_index }
        | State::ResizingCircle { region_index } = self.state
        {
            if region_index < self.scene.regions.len() {
                self.scene.remove_region(region_index);
            }
            self.state = State::Idle;
        }
    }

    fn find_region_at(&self, x: f32, y: f32) -> Option<usize> {
        // Search in reverse order so topmost (most recently added) is found first
        for (i, region) in self.scene.regions.iter().enumerate().rev() {
            if region.contains(x, y) {
                return Some(i);
            }
        }
        None
    }

    fn find_vertex_at(&self, region_index: usize, x: f32, y: f32) -> Option<usize> {
        let region = self.scene.regions.get(region_index)?;
        let click = Point::new(x, y);

        // Only polygons have vertices
        if let Some(Shape::Polygon(poly)) = &region.shape {
            for (i, v) in poly.vertices.iter().enumerate() {
                if click.distance_to(v) < self.vertex_handle_size {
                    return Some(i);
                }
            }
        }
        None
    }

    fn auto_increment_name(&mut self) {
        if let Some(pos) = self.next_region_name.rfind('_') {
            if let Ok(num) = self.next_region_name[pos + 1..].parse::<u32>() {
                self.next_region_name = format!("{}{}", &self.next_region_name[..=pos], num + 1);
            }
        }
    }

    /// Render calibration UI overlay
    pub fn render(&self, buffer: &mut PixelBuffer) {
        let selected_idx = match &self.state {
            State::Selected { region_index }
            | State::DraggingVertex { region_index, .. }
            | State::DraggingCircleCenter { region_index }
            | State::ResizingCircle { region_index } => Some(*region_index),
            _ => None,
        };

        let dragging_vertex = match &self.state {
            State::DraggingVertex {
                region_index,
                vertex_index,
            } => Some((*region_index, *vertex_index)),
            _ => None,
        };

        // Draw existing regions
        for (i, region) in self.scene.regions.iter().enumerate() {
            let is_selected = selected_idx == Some(i);

            let outline_color = if is_selected {
                (100, 150, 255)
            } else {
                (80, 120, 80)
            };

            match region.get_shape() {
                Shape::Polygon(poly) => {
                    // Fill with solid black (masked area)
                    buffer.fill_polygon(&poly.as_tuples(), 0, 0, 0);

                    // Draw outline
                    let vertices = &poly.vertices;
                    if vertices.len() >= 2 {
                        for j in 0..vertices.len() {
                            let p1 = &vertices[j];
                            let p2 = &vertices[(j + 1) % vertices.len()];
                            buffer.line(
                                p1.x as i32,
                                p1.y as i32,
                                p2.x as i32,
                                p2.y as i32,
                                outline_color.0,
                                outline_color.1,
                                outline_color.2,
                            );
                        }
                    }

                    // Draw vertex handles (only for selected polygon)
                    if is_selected {
                        for (vidx, v) in vertices.iter().enumerate() {
                            let is_dragging = dragging_vertex == Some((i, vidx));
                            let is_hovered = !is_dragging
                                && Point::new(self.mouse_pos.0 as f32, self.mouse_pos.1 as f32)
                                    .distance_to(v)
                                    < self.vertex_handle_size;

                            let (size, color) = if is_dragging {
                                (6, (255, 255, 100))
                            } else if is_hovered {
                                (5, (255, 200, 100))
                            } else {
                                (4, (200, 200, 255))
                            };

                            buffer.fill_rect(
                                v.x as i32 - size,
                                v.y as i32 - size,
                                (size * 2 + 1) as u32,
                                (size * 2 + 1) as u32,
                                color.0,
                                color.1,
                                color.2,
                            );
                        }
                    }
                }
                Shape::Circle(circle) => {
                    // Fill with solid black
                    buffer.fill_circle(
                        circle.center.x as i32,
                        circle.center.y as i32,
                        circle.radius as i32,
                        0,
                        0,
                        0,
                    );

                    // Draw outline
                    buffer.draw_circle(
                        circle.center.x as i32,
                        circle.center.y as i32,
                        circle.radius as i32,
                        outline_color.0,
                        outline_color.1,
                        outline_color.2,
                    );

                    // Draw handles for selected circle
                    if is_selected {
                        let mouse = Point::new(self.mouse_pos.0 as f32, self.mouse_pos.1 as f32);

                        // Center handle
                        let dist_to_center = mouse.distance_to(&circle.center);
                        let center_hovered = dist_to_center < self.vertex_handle_size * 2.0;
                        let center_dragging =
                            matches!(self.state, State::DraggingCircleCenter { .. });

                        let (size, color) = if center_dragging {
                            (6, (255, 255, 100))
                        } else if center_hovered {
                            (5, (255, 200, 100))
                        } else {
                            (4, (200, 200, 255))
                        };

                        buffer.fill_rect(
                            circle.center.x as i32 - size,
                            circle.center.y as i32 - size,
                            (size * 2 + 1) as u32,
                            (size * 2 + 1) as u32,
                            color.0,
                            color.1,
                            color.2,
                        );

                        // Edge handle (draw a small square on the right edge)
                        let edge_x = circle.center.x + circle.radius;
                        let edge_y = circle.center.y;
                        let edge_hovered =
                            (mouse.distance_to(&Point::new(edge_x, edge_y)) < self.vertex_handle_size)
                                || ((dist_to_center - circle.radius).abs() < self.vertex_handle_size);
                        let edge_dragging = matches!(self.state, State::ResizingCircle { .. });

                        let (size, color) = if edge_dragging {
                            (6, (255, 255, 100))
                        } else if edge_hovered {
                            (5, (255, 200, 100))
                        } else {
                            (4, (200, 200, 255))
                        };

                        buffer.fill_rect(
                            edge_x as i32 - size,
                            edge_y as i32 - size,
                            (size * 2 + 1) as u32,
                            (size * 2 + 1) as u32,
                            color.0,
                            color.1,
                            color.2,
                        );
                    }
                }
            }
        }

        // Draw in-progress polygon
        if let State::Drawing { vertices } = &self.state {
            // Draw completed edges
            for i in 0..vertices.len().saturating_sub(1) {
                let p1 = &vertices[i];
                let p2 = &vertices[i + 1];
                buffer.line(
                    p1.x as i32,
                    p1.y as i32,
                    p2.x as i32,
                    p2.y as i32,
                    255,
                    220,
                    0,
                );
            }

            // Draw line to cursor
            if let Some(last) = vertices.last() {
                buffer.line(
                    last.x as i32,
                    last.y as i32,
                    self.mouse_pos.0,
                    self.mouse_pos.1,
                    255,
                    220,
                    100,
                );
            }

            // Draw vertices
            for (i, v) in vertices.iter().enumerate() {
                let is_first = i == 0;
                let mouse = Point::new(self.mouse_pos.0 as f32, self.mouse_pos.1 as f32);
                let can_close =
                    is_first && vertices.len() >= 3 && mouse.distance_to(v) < self.snap_distance;

                let (size, color) = if can_close {
                    (6, (0, 255, 100)) // Green highlight when can close
                } else if is_first {
                    (4, (255, 100, 100)) // First vertex is reddish
                } else {
                    (3, (255, 220, 0))
                };

                buffer.fill_rect(
                    v.x as i32 - size,
                    v.y as i32 - size,
                    (size * 2 + 1) as u32,
                    (size * 2 + 1) as u32,
                    color.0,
                    color.1,
                    color.2,
                );
            }
        }

        // Draw in-progress circle
        if let State::DrawingCircle { center } = &self.state {
            let dx = self.mouse_pos.0 as f32 - center.x;
            let dy = self.mouse_pos.1 as f32 - center.y;
            let radius = (dx * dx + dy * dy).sqrt();

            if radius >= 5.0 {
                buffer.draw_circle(
                    center.x as i32,
                    center.y as i32,
                    radius as i32,
                    255,
                    220,
                    0,
                );
            }

            // Draw center point
            buffer.fill_rect(
                center.x as i32 - 4,
                center.y as i32 - 4,
                9,
                9,
                255,
                100,
                100,
            );
        }

        // Draw cursor crosshair
        let (mx, my) = self.mouse_pos;
        buffer.line(mx - 15, my, mx - 5, my, 255, 255, 255);
        buffer.line(mx + 5, my, mx + 15, my, 255, 255, 255);
        buffer.line(mx, my - 15, mx, my - 5, 255, 255, 255);
        buffer.line(mx, my + 5, mx, my + 15, 255, 255, 255);

        // Show shift hint
        if self.shift_held && matches!(self.state, State::Idle | State::Selected { .. }) {
            // Could draw "CIRCLE MODE" text here if desired
        }
    }
}
