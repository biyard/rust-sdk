#![allow(non_snake_case)]
use dioxus::prelude::*;

#[derive(Debug, Clone)]
pub struct ChartTheme {
    pub stack_bar_color_pool: Vec<&'static str>,
    pub horizontal_bar_gradient_colors: Vec<&'static str>,

    // Pie chart renders each pie based on key color and adjust opacity.
    pub pie_chart_colors: Vec<&'static str>,
    pub bar_chart_colors: Vec<&'static str>,
}

impl Default for ChartTheme {
    fn default() -> Self {
        ChartTheme {
            stack_bar_color_pool: vec![
                "#FF8585", "#FF9E37", "#FFDE61", "#9CEF69", "#AAFFEE", "#8D9EF6", "#B4B4B4",
                "#7F7F7F", "#BCBD22", "#17BECF",
            ],
            horizontal_bar_gradient_colors: vec!["#85AEE2", "#2A60D3"],
            pie_chart_colors: vec!["#D4DFF6", "#2A60D3"],
            bar_chart_colors: vec!["#2A60D3", "#85AEE2"],
        }
    }
}

#[derive(Debug, Clone)]
pub struct ColorTheme {
    pub background: String,
    pub icon: IconColorTheme,
    pub button: ButtonColorTheme,
    pub text: TextColorTheme,
    pub card: CardColorTheme,
    pub popup: PopupColorTheme,
    pub input: InputColorTheme,
    pub services: ServiceColorTheme,
}

impl Default for ColorTheme {
    fn default() -> Self {
        ColorTheme {
            background: "#2C2E42".to_string(),
            icon: IconColorTheme::default(),
            button: ButtonColorTheme::default(),
            text: TextColorTheme::default(),
            card: CardColorTheme::default(),
            popup: PopupColorTheme::default(),
            input: InputColorTheme::default(),
            services: ServiceColorTheme::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ServiceColorTheme {
    pub kakao: String,
}

impl Default for ServiceColorTheme {
    fn default() -> Self {
        ServiceColorTheme {
            kakao: "#F7E111".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct IconColorTheme {
    pub primary: String,
    pub secondary: String,
}

impl Default for IconColorTheme {
    fn default() -> Self {
        IconColorTheme {
            primary: "#8588AB".to_string(),
            secondary: "#666C6E".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ButtonColorTheme {
    pub primary: String,
    pub secondary: String,
}

impl Default for ButtonColorTheme {
    fn default() -> Self {
        ButtonColorTheme {
            primary: "#363952".to_string(),
            secondary: "#424563".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TextColorTheme {
    pub primary: String,
    pub secondary: String,
    pub disabled: String,
}

impl Default for TextColorTheme {
    fn default() -> Self {
        TextColorTheme {
            primary: "#FFFFFF".to_string(),
            secondary: "#ADBCD7".to_string(),
            disabled: "#404761".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CardColorTheme {
    pub primary: String,
    pub secondary: String,
}

impl Default for CardColorTheme {
    fn default() -> Self {
        CardColorTheme {
            primary: "#212231".to_string(),
            secondary: "#292B3D".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PopupColorTheme {
    pub background: String,
}

impl Default for PopupColorTheme {
    fn default() -> Self {
        PopupColorTheme {
            background: "#2E3045".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct InputColorTheme {
    pub primary: String,
}

impl Default for InputColorTheme {
    fn default() -> Self {
        InputColorTheme {
            primary: "#212231".to_string(),
        }
    }
}

pub struct BiyardTheme;

impl BiyardTheme {
    pub fn init() {
        use_context_provider(|| ChartTheme::default());
        use_context_provider(|| ColorTheme::default());
        use_context_provider(|| ServiceColorTheme::default());
        use_context_provider(|| IconColorTheme::default());
        use_context_provider(|| ButtonColorTheme::default());
        use_context_provider(|| TextColorTheme::default());
        use_context_provider(|| CardColorTheme::default());
        use_context_provider(|| PopupColorTheme::default());
        use_context_provider(|| InputColorTheme::default());
    }
}
