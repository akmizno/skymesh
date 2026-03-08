use super::Color;

#[derive(Debug)]
pub(crate) struct Reflection {
    ambient: f32,   // 0 - 1
    diffuse: f32,   // 0 - 1
    specular: f32,  // 0 - 1
    shininess: f32, // 0 - 100
}

impl Default for Reflection {
    fn default() -> Self {
        Self::new(
            0.3, // ambient
            0.5, // diffuse
            0.4, // specular
            30., // shininess
        )
    }
}

impl Reflection {
    pub(crate) fn new(ambient: f32, diffuse: f32, specular: f32, shininess: f32) -> Self {
        Self {
            ambient,
            diffuse,
            specular,
            shininess,
        }
    }

    pub(crate) fn ambient(&self) -> f32 {
        self.ambient
    }
    pub(crate) fn diffuse(&self) -> f32 {
        self.diffuse
    }
    pub(crate) fn specular(&self) -> f32 {
        self.specular
    }
    pub(crate) fn shininess(&self) -> f32 {
        self.shininess
    }
}

#[derive(Debug)]
pub(crate) struct Lighting {
    color: Color,
    reflection: Reflection,
}

impl Default for Lighting {
    fn default() -> Self {
        Self::new(Color::from_rgba(1.0, 1.0, 1.0, None), Default::default())
    }
}

impl Lighting {
    pub(crate) fn new(color: Color, reflection: Reflection) -> Self {
        Self { color, reflection }
    }

    pub(crate) fn color(&self) -> &Color {
        &self.color
    }
    pub(crate) fn reflection(&self) -> &Reflection {
        &self.reflection
    }
}
