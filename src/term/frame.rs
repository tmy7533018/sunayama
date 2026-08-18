use super::color::Rgb;

pub struct Frame {
    width: usize,
    height: usize,
    px: Vec<Option<Rgb>>,
}

impl Frame {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            px: vec![None; width * height],
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn clear(&mut self) {
        self.px.fill(None);
    }

    pub fn set(&mut self, x: usize, y: usize, c: Rgb) {
        if x < self.width && y < self.height {
            self.px[y * self.width + x] = Some(c);
        }
    }

    pub fn get(&self, x: usize, y: usize) -> Option<Rgb> {
        if x < self.width && y < self.height {
            self.px[y * self.width + x]
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_empty() {
        let f = Frame::new(4, 4);
        assert_eq!(f.get(0, 0), None);
    }

    #[test]
    fn out_of_bounds_is_clipped_not_wrapped() {
        let mut f = Frame::new(4, 4);
        f.set(4, 0, Rgb::new(1, 2, 3));
        assert_eq!(f.get(0, 1), None);
        assert_eq!(f.get(4, 0), None);
    }
}
