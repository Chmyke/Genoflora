//! Genoflora simulation core: cells that all carry the same genome.
//!
//! Pure and deterministic: no clock, no hash containers, no external crate.
//! The plant shape must never be written here; it has to emerge from local cell rules.

/// Shared, heritable program of every cell. Placeholder until the gene network lands.
#[derive(Clone, Debug, PartialEq)]
pub struct Genome {
    pub genes: Vec<f32>,
}

/// One simplified cell: a sphere with a few chemical concentrations.
#[derive(Clone, Debug, PartialEq)]
pub struct Cell {
    pub position: [f32; 3],
    pub radius: f32,
    pub concentrations: Vec<f32>,
}

/// All living cells plus the genome they share.
#[derive(Clone, Debug, PartialEq)]
pub struct Tissue {
    pub genome: Genome,
    pub cells: Vec<Cell>,
}

impl Tissue {
    /// Starts from a single cell at the origin.
    pub fn seed(genome: Genome, radius: f32, concentrations: Vec<f32>) -> Self {
        let cell = Cell {
            position: [0.0; 3],
            radius,
            concentrations,
        };
        Self {
            genome,
            cells: vec![cell],
        }
    }

    /// Mitosis of cell `index` along `axis` (unit vector): volume and contents are split
    /// evenly between two daughters placed side by side. Returns false for an unknown index.
    pub fn divide(&mut self, index: usize, axis: [f32; 3]) -> bool {
        let Some(mother) = self.cells.get(index).cloned() else {
            return false;
        };
        // Equal volume split: r_daughter = r_mother / cbrt(2).
        let radius = mother.radius / 2f32.cbrt();
        let shift = axis.map(|a| a * radius);
        let mut a = mother.clone();
        a.radius = radius;
        a.position = [0, 1, 2].map(|i| mother.position[i] - shift[i]);
        let mut b = a.clone();
        b.position = [0, 1, 2].map(|i| mother.position[i] + shift[i]);
        self.cells[index] = a;
        self.cells.push(b);
        true
    }

    /// Total cell volume, conserved by division.
    pub fn volume(&self) -> f32 {
        self.cells
            .iter()
            .map(|c| 4.0 / 3.0 * std::f32::consts::PI * c.radius.powi(3))
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tissue() -> Tissue {
        Tissue::seed(
            Genome {
                genes: vec![1.0, 0.5],
            },
            1.0,
            vec![0.2, 0.0],
        )
    }

    #[test]
    fn division_conserves_volume_and_contents() {
        let mut t = tissue();
        let before = t.volume();
        assert!(t.divide(0, [0.0, 0.0, 1.0]));
        assert_eq!(t.cells.len(), 2);
        assert!((t.volume() - before).abs() < 1e-5);
        assert_eq!(t.cells[0].concentrations, t.cells[1].concentrations);
    }

    #[test]
    fn daughters_sit_on_both_sides_of_the_axis() {
        let mut t = tissue();
        t.divide(0, [1.0, 0.0, 0.0]);
        assert!(t.cells[0].position[0] < 0.0 && t.cells[1].position[0] > 0.0);
    }

    #[test]
    fn unknown_cell_is_refused() {
        let mut t = tissue();
        assert!(!t.divide(3, [1.0, 0.0, 0.0]));
        assert_eq!(t, tissue());
    }

    #[test]
    fn same_inputs_same_bits() {
        let run = || {
            let mut t = tissue();
            for i in 0..5 {
                t.divide(i % t.cells.len(), [0.6, 0.0, 0.8]);
            }
            t
        };
        assert_eq!(run(), run());
    }
}
