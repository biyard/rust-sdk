# Package Structure

This document explains what elements should be included when configuring a Front Package and what functions each element performs.

## Directory Structure

The package structure is as follows:

```plaintext
├── components
│    ├── button
│    │    ├── mod.rs
│    ├── checkbox
│    │    ├── mod.rs
│    ├── selectbox
│    │    ├── mod.rs
│    └── mod.rs
├── pages
│    ├── main
│    │    ├── controller.rs
│    │    ├── i18n.rs
│    │    ├── mod.rs
│    │    └── page.rs
│    ├── create
│    │    ├── controller.rs
│    │    ├── i18n.rs
│    │    ├── mod.rs
│    ├──├── page.rs
│    ├── mod.rs
│    ├── layout.rs
│    └── page.rs
├── services
│    ├── user_service.rs
│    └── mod.rs
├── utils
│    ├── api.rs
│    ├── time.rs
│    ├── hash.rs
│    └── mod.rs
├── main.rs
├── routes.rs
├── theme.rs
└── config.rs
```

## Explanation of Structure

### Root Files

- **`main.rs`**  
  It is the entry point of the Dioxus app and is responsible for running the entire application by initializing the logger, registering services, setting themes, loading external resources, and configuring routing.<br>
  The entry point that defines the structure of a Dioxus app and integrates and manages UI layouts, routes, pages, services, components, and utility modules.

  **Example:**

    ```
    pub mod config;
    pub mod pages;
    pub mod routes;

    pub mod service;
    pub mod utils;
    pub mod components;
    ```

- **`routes.rs`**  
  Defines all page paths and language-specific routing structure within the app, and a router configuration for Dioxus that manages page navigation based on the /[:lang] namespace.

  **Example:**

    ```
    use bdk::prelude::;
    use pages::;

    #[derive(Clone, Routable, Debug, PartialEq)]
    #[rustfmt::skip]
    pub enum Route {
        #[nest("/:lang")]
            #[layout(RootLayout)]
                #[route("/main")]
                MainPage { lang: Language },
                #[route("/create")]
                CreatePage { lang: Language },
            #[end_layout]
        #[end_nest]

        #[route("/:..route")]
        NotFoundPage { route: Vec<String> },
    }

    ```

- **`config.rs`**  
  A configuration module that initializes environment variable-based settings (Config) when the app runs and provides them in a globally accessible singleton form.

- **`theme.rs`**  
  A file that defines global theme data and manages the application of colors and font styles consistently across the UI via Context.

  **Example:**

    ```
    #[derive(Debug, Clone)]
    pub struct ThemeData {
        pub active: String,
        pub active00: String,
        pub font_theme: FontTheme,
    }

    #[derive(Debug, Clone)]
    pub struct FontTheme {
        pub exbold15: String,
        pub bold15: String,
    }

    impl Default for FontTheme {
        fn default() -> Self {
            FontTheme {
                exbold15: "font-extrabold text-[15px] leading-[22.5px]".to_string(),
                bold15: "font-bold text-[15px] leading[22.5px]".to_string(),
            }
        }
    }

    impl Default for ThemeData {
        fn default() -> Self {
            ThemeData {
                active: "#68D36C".to_string(), 
                active00: "#68D36C".to_string(), 
                font_theme: FontTheme::default(),
            }
        }
    }

    use bdk::prelude::*;

    #[derive(Debug, Clone, Copy, Default)]
    pub struct Theme {
        pub data: Signal<ThemeData>,
    }

    impl Theme {
        pub fn init() {
            use_context_provider(|| Self {
                data: Signal::new(ThemeData::default()),
            });
        }

        pub fn get_data(&self) -> ThemeData {
            (self.data)()
        }

        pub fn get_font_theme(&self) -> FontTheme {
            (self.data)().font_theme.clone()
        }
    }

    ```

### components
- Create UI by organizing modules that can be commonly used for each page into a components directory.
- Components are configured by creating a directory with each component name and creating a mod.rs file inside that directory.

**Example:**

If the Components is defined as:
```
├── components
│    ├── button
│    │    ├── mod.rs
│    ├── checkbox
│    │    ├── mod.rs
│    ├── selectbox
│    │    ├── mod.rs
│    └── mod.rs
```

At this time, the mod.rs file should be structured as follows:
```
pub mod button;
pub mod checkbox;
pub mod selectbox;
```

### services
- Configure services for functions that operate commonly within a page.

#### step
The steps to configure and use the service are as follows: <br>
1. service file setting
- You can define a service file as follows:
``` user_service.rs
use bdk::prelude::*;

#[derive(Debug, Clone, Copy, Default)]
pub struct UserService {
    pub id: Signal<Option<String>>,
}

impl UserService {
    pub fn init() {
        let srv = Self {
            id: Signal::new(None),
        };
        use_context_provider(|| srv);
    }
}
``` 

2. Initialize in main.rs file
- Services are initialized globally in main.rs and are set up so that they can be used commonly by each page component.
- Service initialization is done as follows:
``` main.rs
use bdk::prelude::*;

fn main() {
    dioxus_logger::init(config::get().log_level).expect("failed to init logger");

    dioxus_aws::launch(App);
}

#[component]
fn App() -> Element {
    UserService::init();

    rsx! {
        Router::<Route> {}
    }
}

```

3. accesssed via use_context
- In each page's controller.rs, services are accessed via use_context, allowing for seamless dependency injection and modular usage.
- Services can be accessed as follows:
``` controller.rs
use bdk::prelude::;

#[derive(Debug, Clone, Copy, DioxusController)]
pub struct Controller {
    user_service: UserService,
}

impl Controller {
    pub fn new(
        lang: Language,
    ) -> std::result::Result<Self, RenderError> {
        let ctrl = Self {
            user_service: use_context(),
        };

        Ok(ctrl)
    }
}
```

### utils
- A space where utility functions that can be commonly used on each page are collected.
- Modularize and manage general logic such as time formatting (time.rs) or hash generation (hash.rs).

**Example:**

If the Utils is defined as:
```
├── utils
│    ├── api.rs
│    ├── time.rs
│    ├── hash.rs
│    └── mod.rs
```

### pages
- Each page directory name must match the routing name.
- Each directory consists of controller.rs, i18n.rs, mod.rs, and page.rs.
- The page mainly utilizes the MVC pattern, the model is managed in the dto package, the ui is managed in the page.rs file, and the functional logic is managed in the controller.rs file.
- Components that are only used within a page must be used by configuring a separate components directory within the directory.

**Example:**

If the pages is defined as:
``` /main
├── main
│    ├── components
│    │    ├── mod.rs
│    │    ├── objective_quiz.rs
│    │    └── subjective_quiz.rs
│    ├── controller.rs
│    ├── i18n.rs
│    ├── mod.rs
│    └── page.rs
├── layout.rs
└── mod.rs
```

#### controller.rs
- Mainly creating functional logic.
- The pattern used is as follows:

**controller.rs**
``` controller.rs
use bdk::prelude::*;

#[derive(Debug, Clone, Copy, DioxusController)]
pub struct Controller {
    count: Signal<i64>,
    lang: Language,
}

impl Controller {
    pub fn new(lang: Language) -> std::result::Result<Self, RenderError> {
        let count = use_signal(|| 0);
        let ctrl = Self { lang, count };
        Ok(ctrl)
    }

    pub fn add_count(&mut self) {
        let mut c = (self.count)();
        c += 1;
        (self.count).set(c);
    }

    pub fn get_count(&self) -> i64 {
        (self.count)()
    }
}

```

**page.rs**
``` page.rs
use bdk::prelude::*;

#[component]
pub fn MainPage(lang: Language) -> Element {
    let ctrl = Controller::new(lang)?;
    let count = ctrl.get_count();

    rsx! {
        div {
            "main page"
        }
    }
}
```

#### i18n.rs
- Translate configuration using the translate macro, which is an internally implemented macro.
- Basically, ko and en are supported in two versions, and the two versions are described in the i18n.rs file when configuring the UI.
- At this time, the Translation name should reflect the page name.
- Examples of usage are as follows.

**i18n.rs**
``` i18n.rs
use bdk::prelude::*;

translate! {
    MainTranslate;

    text: {
        ko: "메인 페이지",
        en: "Main Page"
    }
}

```

**page.rs**
``` page.rs
use bdk::prelude::*;

#[component]
pub fn MainPage(lang: Language) -> Element {
    let ctrl = Controller::new(lang);
    let tr: MainTranslate = translate(&lang);

    let count = ctrl.get_count();
    let text = tr.text;

    rsx! {
        div {
            "main page"
        }
    }
}
```

#### page.rs
- page.rs is defined around UI composition and screen elements shown to the user, rather than business logic.
- Business logic is configured in the controller.rs file, and translation-related parts are configured in the i18n.rs file.