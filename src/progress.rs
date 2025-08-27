use std::{io, time::Duration};

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use npins::nix::FetchStatus;
use tokio::time::interval;

pub struct ProgressUI {
    multiprogress: MultiProgress,
    status: ProgressBar,
}

pub struct PinProgress {
    multiprogress: MultiProgress,
    status: ProgressBar,
    progressbar: ProgressBar,
}

impl ProgressUI {
    pub fn new(length: u64) -> Self {
        let multiprogress = MultiProgress::new();
        let status = multiprogress.add(
            ProgressBar::new(length)
                .with_style(ProgressStyle::with_template("{pos}/{len} {elapsed}").unwrap()),
        );

        let weak_status = status.downgrade();
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(1));

            while let Some(status) = weak_status.upgrade() {
                status.tick();
                drop(status);
                interval.tick().await;
            }
        });

        Self {
            multiprogress,
            status,
        }
    }

    pub fn add_pin(&self, name: String) -> PinProgress {
        let progressbar = self
            .multiprogress
            .insert_from_back(
                1,
                ProgressBar::no_length()
                    .with_style(ProgressStyle::with_template("{prefix:.3} {msg}").unwrap()),
            )
            .with_prefix(name);

        progressbar.tick();

        PinProgress {
            multiprogress: self.multiprogress.clone(),
            status: self.status.clone(),
            progressbar,
        }
    }
}

impl PinProgress {
    pub fn write(&self, msg: &str) -> io::Result<()> {
        self.multiprogress.println(msg)
    }

    pub fn fetch_status_callback(&self) -> Box<dyn FnMut(FetchStatus) + Send> {
        let pb = self.progressbar.clone();
        Box::new(move |status| {
            process_fetch(&pb, status);
        })
    }
}

fn process_fetch(pb: &ProgressBar, status: FetchStatus) {
    let (downloaded, total) = match status {
        FetchStatus::Progress { downloaded, total } => (downloaded, total),
        FetchStatus::Message(message) => {
            pb.set_message(message);
            return;
        },
    };

    if pb.length().is_none() {
        let template = if total != 0 {
            "{prefix:.3} {bar} {bytes}/{total_bytes} {eta} {msg}"
        } else {
            "{prefix:.3} {bytes} {msg}"
        };
        pb.set_style(ProgressStyle::with_template(template).unwrap());
    }

    // Nix will give us a total length after the whole thing is downloaded
    // We do not want to set the length as to prevent the bar from showing up
    if pb.length() != Some(total) && total != downloaded {
        pb.set_length(total);
    }

    pb.set_position(downloaded);

    // This might get called multiple times but it won't matter
    if downloaded == total {
        let template = if pb.length() != Some(0) {
            "{prefix:.3} {bar} {bytes}/{total_bytes} {msg}"
        } else {
            "{prefix:.3} {bytes} {msg}"
        };
        pb.set_style(ProgressStyle::with_template(template).unwrap());
        // pb.force_draw();
    }
}

impl Drop for ProgressUI {
    fn drop(&mut self) {
        self.multiprogress.clear().unwrap();
    }
}

impl Drop for PinProgress {
    fn drop(&mut self) {
        self.multiprogress.remove(&self.progressbar);
        self.status.inc(1);
    }
}
