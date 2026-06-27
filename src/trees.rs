#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TreeType {
    Oak,
    Pine,
    Birch,
    Palm,
    Bush,
}

// The shared climate constants
const DESERT_TEMP_MIN: f64 = 0.75;
const DESERT_MOIST_MAX: f64 = 0.25;
const MOUNTAIN_TEMP_MAX: f64 = 0.35;
const WET_HILLS_TEMP_MIN: f64 = 0.6;
const WET_HILLS_MOIST_MIN: f64 = 0.7;
const BIRCH_MOIST_MIN: f64 = 0.55;

/// Single source of truth for tree selection
pub fn get_suitable_trees(
    temp: f64,
    moist: f64,
    height: f64,
    altitude_jitter: f64,
) -> Vec<TreeType> {
    let mut trees = Vec::new();

    if temp > DESERT_TEMP_MIN && moist < DESERT_MOIST_MAX {
        trees.push(TreeType::Palm);
    } else if temp < MOUNTAIN_TEMP_MAX && height > crate::terrain::PINE_ALTITUDE + altitude_jitter {
        trees.push(TreeType::Pine);
    } else if temp >= WET_HILLS_TEMP_MIN && moist >= WET_HILLS_MOIST_MIN {
        trees.push(TreeType::Oak);
    } else if moist > BIRCH_MOIST_MIN {
        trees.push(TreeType::Birch);
    }

    if !(temp > DESERT_TEMP_MIN && moist < DESERT_MOIST_MAX)
        && height < crate::terrain::BUSH_MAX_HEIGHT
    {
        trees.push(TreeType::Bush);
    }

    trees
}

/// Single source of truth for vegetation spacing/density
pub fn get_vegetation_radius(temp: f64, moist: f64, height: f64, near_water: bool) -> f32 {
    if temp > DESERT_TEMP_MIN && moist < DESERT_MOIST_MAX {
        return if near_water { 15.0 } else { f32::INFINITY };
    }

    if temp < MOUNTAIN_TEMP_MAX && height > crate::terrain::PINE_ALTITUDE {
        return 12.0;
    }

    if temp >= WET_HILLS_TEMP_MIN && moist >= WET_HILLS_MOIST_MIN {
        return 20.0;
    }

    if moist > BIRCH_MOIST_MIN {
        return 6.0;
    }

    let clump_factor = (moist - 0.45).abs();
    12.0 + (clump_factor * 200.0) as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_desert_palms() {
        let trees = get_suitable_trees(0.8, 0.1, 10.0, 0.0);
        assert_eq!(trees, vec![TreeType::Palm]);
    }

    #[test]
    fn test_high_mountains_pine() {
        let trees = get_suitable_trees(0.2, 0.4, 200.0, 0.0);
        assert!(trees.contains(&TreeType::Pine));
        assert!(!trees.contains(&TreeType::Bush)); // Height > BUSH_MAX_HEIGHT
    }

    #[test]
    fn test_low_mountains_bush() {
        let trees = get_suitable_trees(0.2, 0.4, 10.0, 0.0);
        assert!(!trees.contains(&TreeType::Pine));
        assert!(trees.contains(&TreeType::Bush));
    }

    #[test]
    fn test_wet_hills_oak_and_bush() {
        let trees = get_suitable_trees(0.8, 0.8, 10.0, 0.0);
        assert!(trees.contains(&TreeType::Oak));
        assert!(trees.contains(&TreeType::Bush));
        assert!(!trees.contains(&TreeType::Birch)); // Mutually exclusive with Oak
    }

    #[test]
    fn test_moderate_hills_birch_and_bush() {
        let trees = get_suitable_trees(0.8, 0.6, 10.0, 0.0);
        assert!(trees.contains(&TreeType::Birch));
        assert!(trees.contains(&TreeType::Bush));
        assert!(!trees.contains(&TreeType::Oak)); // Mutually exclusive with Birch
    }

    #[test]
    fn test_plains_bush_only() {
        let trees = get_suitable_trees(0.5, 0.5, 10.0, 0.0);
        assert_eq!(trees, vec![TreeType::Bush]);
    }
}
