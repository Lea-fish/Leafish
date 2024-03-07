use crate::{resources::Manager, ui::glow::ui::{HAttach, ImageBuilder, TextBuilder, VAttach}};

use super::{Container, ImageRef};


#[derive(Default)]
pub struct ManagerUI {
    progress_ui: Vec<ProgressUI>,
    num_tasks: isize,
}

impl ManagerUI {

    pub fn tick(&mut self, manager: &Manager, ui_container: &mut Container, delta: f64) {
        const UI_HEIGHT: f64 = 32.0;

        let delta = delta.min(5.0);

        // Find out what we have to work with
        for task in &manager.vanilla_progress.lock().unwrap().tasks {
            if !self
                .progress_ui
                .iter()
                .filter(|v| v.task_file == task.task_file)
                .any(|v| v.task_name == task.task_name)
            {
                self.num_tasks += 1;
                // Add a ui element for it
                let background = ImageBuilder::new()
                    .texture("leafish:solid")
                    .position(0.0, -UI_HEIGHT)
                    .size(350.0, UI_HEIGHT)
                    .colour((0, 0, 0, 100))
                    .draw_index(0xFFFFFF - self.num_tasks)
                    .alignment(VAttach::Bottom, HAttach::Left)
                    .create(ui_container);

                ImageBuilder::new()
                    .texture("leafish:solid")
                    .position(0.0, 0.0)
                    .size(350.0, 10.0)
                    .colour((0, 0, 0, 200))
                    .attach(&mut *background.borrow_mut());
                TextBuilder::new()
                    .text(&*task.task_name)
                    .position(3.0, 0.0)
                    .scale_x(0.5)
                    .scale_y(0.5)
                    .draw_index(1)
                    .attach(&mut *background.borrow_mut());
                TextBuilder::new()
                    .text(&*task.task_file)
                    .position(3.0, 12.0)
                    .scale_x(0.5)
                    .scale_y(0.5)
                    .draw_index(1)
                    .attach(&mut *background.borrow_mut());

                let progress_bar = ImageBuilder::new()
                    .texture("leafish:solid")
                    .position(0.0, 0.0)
                    .size(0.0, 10.0)
                    .colour((0, 255, 0, 255))
                    .draw_index(2)
                    .alignment(VAttach::Bottom, HAttach::Left)
                    .attach(&mut *background.borrow_mut());

                self.progress_ui.push(ProgressUI {
                    task_name: task.task_name.clone(),
                    task_file: task.task_file.clone(),
                    position: -UI_HEIGHT,
                    closing: false,
                    progress: 0.0,
                    background,
                    progress_bar,
                });
            }
        }
        for ui in &mut self.progress_ui {
            if ui.closing {
                continue;
            }
            let mut found = false;
            let mut prog = 1.0;
            for task in manager.vanilla_progress.lock().unwrap()
                .tasks
                .iter()
                .filter(|v| v.task_file == ui.task_file)
                .filter(|v| v.task_name == ui.task_name)
            {
                found = true;
                prog = task.progress as f64 / task.total as f64;
            }
            let background = ui.background.borrow();
            let progress_bar = ui.progress_bar.borrow();
            // Let the progress bar finish
            if !found
                && (background.y - ui.position).abs() < 0.7 * delta
                && (progress_bar.width - 350.0).abs() < 1.0 * delta
            {
                ui.closing = true;
                ui.position = -UI_HEIGHT;
            }
            ui.progress = prog;
        }
        let mut offset = 0.0;
        for ui in &mut self.progress_ui {
            if ui.closing {
                continue;
            }
            ui.position = offset;
            offset += UI_HEIGHT;
        }
        // Move elements
        for ui in &mut self.progress_ui {
            let mut background = ui.background.borrow_mut();
            if (background.y - ui.position).abs() < 0.7 * delta {
                background.y = ui.position;
            } else {
                background.y += (ui.position - background.y).signum() * 0.7 * delta;
            }
            let mut progress_bar = ui.progress_bar.borrow_mut();
            let target_size = (350.0 * ui.progress).min(350.0);
            if (progress_bar.width - target_size).abs() < 1.0 * delta {
                progress_bar.width = target_size;
            } else {
                progress_bar.width +=
                    ((target_size - progress_bar.width).signum() * delta).max(0.0);
            }
        }

        // Clean up dead elements
        self.progress_ui
            .retain(|v| v.position >= -UI_HEIGHT || !v.closing);
    }

}

struct ProgressUI {
    task_name: String,
    task_file: String,
    position: f64,
    closing: bool,
    progress: f64,

    background: ImageRef,
    progress_bar: ImageRef,
}