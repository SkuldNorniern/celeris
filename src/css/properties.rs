//! CSS property definitions and categorization

use super::values::{Value, Color};

/// Known CSS property names for better type safety and validation
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Property {
    // Layout
    Display,
    Position,
    Width,
    Height,
    MaxWidth,
    MaxHeight,
    MinWidth,
    MinHeight,

    // Box Model
    Margin,
    MarginTop,
    MarginRight,
    MarginBottom,
    MarginLeft,
    Padding,
    PaddingTop,
    PaddingRight,
    PaddingBottom,
    PaddingLeft,
    Border,
    BorderWidth,
    BorderStyle,
    BorderColor,
    BorderRadius,

    // Flexbox
    FlexDirection,
    FlexWrap,
    JustifyContent,
    AlignItems,
    AlignContent,
    FlexGrow,
    FlexShrink,
    FlexBasis,
    Gap,

    // Grid
    GridTemplateColumns,
    GridTemplateRows,
    GridColumn,
    GridRow,
    GridArea,

    // Typography
    FontFamily,
    FontSize,
    FontWeight,
    LineHeight,
    Color,
    TextAlign,
    TextDecoration,
    TextTransform,
    LetterSpacing,
    WordSpacing,

    // Background
    BackgroundColor,
    BackgroundImage,
    BackgroundSize,
    BackgroundPosition,
    BackgroundRepeat,

    // Transform & Animation
    Transform,
    TransformOrigin,
    Transition,
    Animation,
    AnimationName,
    AnimationDuration,
    AnimationTimingFunction,
    AnimationDelay,
    AnimationIterationCount,
    AnimationDirection,
    AnimationFillMode,
    AnimationPlayState,

    // Positioning
    Top,
    Right,
    Bottom,
    Left,
    ZIndex,

    // Visual
    Opacity,
    Visibility,
    Overflow,
    OverflowX,
    OverflowY,
    Cursor,
    BoxShadow,
    TextShadow,

    // List
    ListStyle,
    ListStyleType,
    ListStylePosition,
    ListStyleImage,

    // Table
    TableLayout,
    BorderCollapse,
    BorderSpacing,

    // Generated Content
    Content,
    Quotes,
    CounterIncrement,
    CounterReset,

    // Other
    VerticalAlign,
    WhiteSpace,
    WordBreak,
    TextOverflow,

    // Custom/Unknown
    Custom(String),
}

impl Property {
    /// Convert string property name to Property enum
    pub fn from_string(name: &str) -> Self {
        match name.to_lowercase().as_str() {
            // Layout
            "display" => Property::Display,
            "position" => Property::Position,
            "width" => Property::Width,
            "height" => Property::Height,
            "max-width" => Property::MaxWidth,
            "max-height" => Property::MaxHeight,
            "min-width" => Property::MinWidth,
            "min-height" => Property::MinHeight,

            // Box Model
            "margin" => Property::Margin,
            "margin-top" => Property::MarginTop,
            "margin-right" => Property::MarginRight,
            "margin-bottom" => Property::MarginBottom,
            "margin-left" => Property::MarginLeft,
            "padding" => Property::Padding,
            "padding-top" => Property::PaddingTop,
            "padding-right" => Property::PaddingRight,
            "padding-bottom" => Property::PaddingBottom,
            "padding-left" => Property::PaddingLeft,
            "border" => Property::Border,
            "border-width" => Property::BorderWidth,
            "border-style" => Property::BorderStyle,
            "border-color" => Property::BorderColor,
            "border-radius" => Property::BorderRadius,

            // Flexbox
            "flex-direction" => Property::FlexDirection,
            "flex-wrap" => Property::FlexWrap,
            "justify-content" => Property::JustifyContent,
            "align-items" => Property::AlignItems,
            "align-content" => Property::AlignContent,
            "flex-grow" => Property::FlexGrow,
            "flex-shrink" => Property::FlexShrink,
            "flex-basis" => Property::FlexBasis,
            "gap" => Property::Gap,

            // Grid
            "grid-template-columns" => Property::GridTemplateColumns,
            "grid-template-rows" => Property::GridTemplateRows,
            "grid-column" => Property::GridColumn,
            "grid-row" => Property::GridRow,
            "grid-area" => Property::GridArea,

            // Typography
            "font-family" => Property::FontFamily,
            "font-size" => Property::FontSize,
            "font-weight" => Property::FontWeight,
            "line-height" => Property::LineHeight,
            "color" => Property::Color,
            "text-align" => Property::TextAlign,
            "text-decoration" => Property::TextDecoration,
            "text-transform" => Property::TextTransform,
            "letter-spacing" => Property::LetterSpacing,
            "word-spacing" => Property::WordSpacing,

            // Background
            "background-color" => Property::BackgroundColor,
            "background-image" => Property::BackgroundImage,
            "background-size" => Property::BackgroundSize,
            "background-position" => Property::BackgroundPosition,
            "background-repeat" => Property::BackgroundRepeat,

            // Transform & Animation
            "transform" => Property::Transform,
            "transform-origin" => Property::TransformOrigin,
            "transition" => Property::Transition,
            "animation" => Property::Animation,
            "animation-name" => Property::AnimationName,
            "animation-duration" => Property::AnimationDuration,
            "animation-timing-function" => Property::AnimationTimingFunction,
            "animation-delay" => Property::AnimationDelay,
            "animation-iteration-count" => Property::AnimationIterationCount,
            "animation-direction" => Property::AnimationDirection,
            "animation-fill-mode" => Property::AnimationFillMode,
            "animation-play-state" => Property::AnimationPlayState,

            // Positioning
            "top" => Property::Top,
            "right" => Property::Right,
            "bottom" => Property::Bottom,
            "left" => Property::Left,
            "z-index" => Property::ZIndex,

            // Visual
            "opacity" => Property::Opacity,
            "visibility" => Property::Visibility,
            "overflow" => Property::Overflow,
            "overflow-x" => Property::OverflowX,
            "overflow-y" => Property::OverflowY,
            "cursor" => Property::Cursor,
            "box-shadow" => Property::BoxShadow,
            "text-shadow" => Property::TextShadow,

            // List
            "list-style" => Property::ListStyle,
            "list-style-type" => Property::ListStyleType,
            "list-style-position" => Property::ListStylePosition,
            "list-style-image" => Property::ListStyleImage,

            // Table
            "table-layout" => Property::TableLayout,
            "border-collapse" => Property::BorderCollapse,
            "border-spacing" => Property::BorderSpacing,

            // Generated Content
            "content" => Property::Content,
            "quotes" => Property::Quotes,
            "counter-increment" => Property::CounterIncrement,
            "counter-reset" => Property::CounterReset,

            // Other
            "vertical-align" => Property::VerticalAlign,
            "white-space" => Property::WhiteSpace,
            "word-break" => Property::WordBreak,
            "text-overflow" => Property::TextOverflow,

            // Custom/Unknown
            _ => Property::Custom(name.to_string()),
        }
    }

    /// Convert Property enum back to string
    pub fn to_string(&self) -> String {
        match self {
            // Layout
            Property::Display => "display".to_string(),
            Property::Position => "position".to_string(),
            Property::Width => "width".to_string(),
            Property::Height => "height".to_string(),
            Property::MaxWidth => "max-width".to_string(),
            Property::MaxHeight => "max-height".to_string(),
            Property::MinWidth => "min-width".to_string(),
            Property::MinHeight => "min-height".to_string(),

            // Box Model
            Property::Margin => "margin".to_string(),
            Property::MarginTop => "margin-top".to_string(),
            Property::MarginRight => "margin-right".to_string(),
            Property::MarginBottom => "margin-bottom".to_string(),
            Property::MarginLeft => "margin-left".to_string(),
            Property::Padding => "padding".to_string(),
            Property::PaddingTop => "padding-top".to_string(),
            Property::PaddingRight => "padding-right".to_string(),
            Property::PaddingBottom => "padding-bottom".to_string(),
            Property::PaddingLeft => "padding-left".to_string(),
            Property::Border => "border".to_string(),
            Property::BorderWidth => "border-width".to_string(),
            Property::BorderStyle => "border-style".to_string(),
            Property::BorderColor => "border-color".to_string(),
            Property::BorderRadius => "border-radius".to_string(),

            // Flexbox
            Property::FlexDirection => "flex-direction".to_string(),
            Property::FlexWrap => "flex-wrap".to_string(),
            Property::JustifyContent => "justify-content".to_string(),
            Property::AlignItems => "align-items".to_string(),
            Property::AlignContent => "align-content".to_string(),
            Property::FlexGrow => "flex-grow".to_string(),
            Property::FlexShrink => "flex-shrink".to_string(),
            Property::FlexBasis => "flex-basis".to_string(),
            Property::Gap => "gap".to_string(),

            // Grid
            Property::GridTemplateColumns => "grid-template-columns".to_string(),
            Property::GridTemplateRows => "grid-template-rows".to_string(),
            Property::GridColumn => "grid-column".to_string(),
            Property::GridRow => "grid-row".to_string(),
            Property::GridArea => "grid-area".to_string(),

            // Typography
            Property::FontFamily => "font-family".to_string(),
            Property::FontSize => "font-size".to_string(),
            Property::FontWeight => "font-weight".to_string(),
            Property::LineHeight => "line-height".to_string(),
            Property::Color => "color".to_string(),
            Property::TextAlign => "text-align".to_string(),
            Property::TextDecoration => "text-decoration".to_string(),
            Property::TextTransform => "text-transform".to_string(),
            Property::LetterSpacing => "letter-spacing".to_string(),
            Property::WordSpacing => "word-spacing".to_string(),

            // Background
            Property::BackgroundColor => "background-color".to_string(),
            Property::BackgroundImage => "background-image".to_string(),
            Property::BackgroundSize => "background-size".to_string(),
            Property::BackgroundPosition => "background-position".to_string(),
            Property::BackgroundRepeat => "background-repeat".to_string(),

            // Transform & Animation
            Property::Transform => "transform".to_string(),
            Property::TransformOrigin => "transform-origin".to_string(),
            Property::Transition => "transition".to_string(),
            Property::Animation => "animation".to_string(),
            Property::AnimationName => "animation-name".to_string(),
            Property::AnimationDuration => "animation-duration".to_string(),
            Property::AnimationTimingFunction => "animation-timing-function".to_string(),
            Property::AnimationDelay => "animation-delay".to_string(),
            Property::AnimationIterationCount => "animation-iteration-count".to_string(),
            Property::AnimationDirection => "animation-direction".to_string(),
            Property::AnimationFillMode => "animation-fill-mode".to_string(),
            Property::AnimationPlayState => "animation-play-state".to_string(),

            // Positioning
            Property::Top => "top".to_string(),
            Property::Right => "right".to_string(),
            Property::Bottom => "bottom".to_string(),
            Property::Left => "left".to_string(),
            Property::ZIndex => "z-index".to_string(),

            // Visual
            Property::Opacity => "opacity".to_string(),
            Property::Visibility => "visibility".to_string(),
            Property::Overflow => "overflow".to_string(),
            Property::OverflowX => "overflow-x".to_string(),
            Property::OverflowY => "overflow-y".to_string(),
            Property::Cursor => "cursor".to_string(),
            Property::BoxShadow => "box-shadow".to_string(),
            Property::TextShadow => "text-shadow".to_string(),

            // List
            Property::ListStyle => "list-style".to_string(),
            Property::ListStyleType => "list-style-type".to_string(),
            Property::ListStylePosition => "list-style-position".to_string(),
            Property::ListStyleImage => "list-style-image".to_string(),

            // Table
            Property::TableLayout => "table-layout".to_string(),
            Property::BorderCollapse => "border-collapse".to_string(),
            Property::BorderSpacing => "border-spacing".to_string(),

            // Generated Content
            Property::Content => "content".to_string(),
            Property::Quotes => "quotes".to_string(),
            Property::CounterIncrement => "counter-increment".to_string(),
            Property::CounterReset => "counter-reset".to_string(),

            // Other
            Property::VerticalAlign => "vertical-align".to_string(),
            Property::WhiteSpace => "white-space".to_string(),
            Property::WordBreak => "word-break".to_string(),
            Property::TextOverflow => "text-overflow".to_string(),

            // Custom/Unknown
            Property::Custom(name) => name.clone(),
        }
    }

    /// Check if this property accepts multiple values (like margin, padding)
    pub fn accepts_multiple_values(&self) -> bool {
        matches!(
            self,
            Property::Margin
                | Property::Padding
                | Property::BorderWidth
                | Property::BorderColor
                | Property::BorderStyle
                | Property::BorderRadius
                | Property::BackgroundPosition
                | Property::BackgroundSize
        )
    }

    /// Check if this property can be inherited
    pub fn is_inherited(&self) -> bool {
        matches!(
            self,
            Property::Color
                | Property::FontFamily
                | Property::FontSize
                | Property::FontWeight
                | Property::LineHeight
                | Property::TextAlign
                | Property::TextTransform
                | Property::LetterSpacing
                | Property::WordSpacing
                | Property::WhiteSpace
                | Property::Visibility
                | Property::Cursor
        )
    }
}


