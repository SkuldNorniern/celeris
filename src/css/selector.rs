#[derive(Clone, Debug, PartialEq)]
pub struct Selector {
    pub components: Vec<SelectorComponent>,
    pub specificity: Specificity,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SelectorComponent {
    Type(String),
    Id(String),
    Class(String),
    Universal,
    Descendant,
    Child,
    Adjacent,
    PseudoClass(String), // :hover, :active, :first-child, etc.
    PseudoElement(String), // ::before, ::after
    Attribute(String, Option<String>), // [attr], [attr=value], [attr~=value], etc.
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Specificity(pub u32, pub u32, pub u32);

impl Selector {
    pub fn new(components: Vec<SelectorComponent>) -> Self {
        let specificity = Self::calculate_specificity(&components);
        Self {
            components,
            specificity,
        }
    }

    fn calculate_specificity(components: &[SelectorComponent]) -> Specificity {
        let mut ids = 0;
        let mut classes = 0;
        let mut types = 0;

        for component in components {
            match component {
                SelectorComponent::Id(_) => ids += 1,
                SelectorComponent::Class(_) => classes += 1,
                SelectorComponent::Type(_) => types += 1,
                _ => {}
            }
        }

        Specificity(ids, classes, types)
    }
}
