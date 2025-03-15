use dioxus::prelude::*;

pub struct UseScroll {
    pub task: CopyValue<Task>,
    pub callback: Callback<(), Task>,
}

pub fn use_scroll<F>(mut callback: F)
where
    F: FnMut(f64, f64) + 'static + Copy,
{
    let callback = use_callback(move |_| {
        let script = r#"
            window.addEventListener('scroll', () => {
                dioxus.send(`${window.scrollY},${window.scrollX}`);
            });
        "#;
        let mut eval = document::eval(script);

        spawn(async move {
            loop {
                let pos = eval.recv::<String>().await.unwrap_or_default();
                tracing::debug!("pos: {}", pos);
                let mut pos = pos.split(',');
                let y = pos
                    .next()
                    .unwrap_or_default()
                    .parse::<f64>()
                    .unwrap_or_default();
                let x = pos
                    .next()
                    .unwrap_or_default()
                    .parse::<f64>()
                    .unwrap_or_default();

                callback(x, y);
            }
        })
    });

    let task = use_hook(|| CopyValue::new(callback(())));

    UseScroll { task, callback };
}
