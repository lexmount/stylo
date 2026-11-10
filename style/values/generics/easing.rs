/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

//! Generic types for CSS Easing Functions.
//! https://drafts.csswg.org/css-easing/#timing-functions

use crate::derives::*;
use std::fmt::{self, Write};
use style_traits::{CssWriter, ToCss};

/// A generic easing function.
#[derive(
    Clone,
    Debug,
    MallocSizeOf,
    PartialEq,
    SpecifiedValueInfo,
    ToShmem,
    ToTyped,
    Serialize,
    Deserialize,
)]
#[value_info(ty = "TIMING_FUNCTION")]
#[repr(u8, C)]
pub enum TimingFunction<Integer, Number, LinearStops> {
    /// `linear | ease | ease-in | ease-out | ease-in-out`
    Keyword(TimingKeyword),
    /// `cubic-bezier(<number>, <number>, <number>, <number>)`
    #[allow(missing_docs)]
    #[css(comma, function)]
    #[typed(skip)]
    CubicBezier {
        x1: Number,
        y1: Number,
        x2: Number,
        y2: Number,
    },
    /// `steps(<integer>, [ <step-position> ]?)`
    /// `<step-position> = jump-start | jump-end | jump-none | jump-both | start | end`
    #[css(comma, function)]
    #[typed(skip)]
    Steps(Integer, #[css(skip_if = "is_end")] StepPosition),
    /// linear([<linear-stop>]#)
    /// <linear-stop> = <output> && <linear-stop-length>?
    /// <linear-stop-length> = <percentage>{1, 2}
    #[css(function = "linear")]
    #[typed(skip)]
    LinearFunction(LinearStops),
    /// The specified `step-start` keyword, before computed-value expansion.
    StepStart,
    /// The specified `step-end` keyword, before computed-value expansion.
    StepEnd,
}

impl<Integer: ToCss, Number: ToCss, LinearStops: ToCss> ToCss
    for TimingFunction<Integer, Number, LinearStops>
{
    fn to_css<W>(&self, dest: &mut CssWriter<W>) -> fmt::Result
    where
        W: fmt::Write,
    {
        match self {
            Self::Keyword(keyword) => keyword.to_css(dest),
            Self::StepStart => dest.write_str("steps(1, start)"),
            Self::StepEnd => dest.write_str("steps(1)"),
            Self::CubicBezier { x1, y1, x2, y2 } => {
                dest.write_str("cubic-bezier(")?;
                x1.to_css(dest)?;
                dest.write_str(", ")?;
                y1.to_css(dest)?;
                dest.write_str(", ")?;
                x2.to_css(dest)?;
                dest.write_str(", ")?;
                y2.to_css(dest)?;
                dest.write_char(')')
            },
            Self::Steps(steps, position) => {
                dest.write_str("steps(")?;
                steps.to_css(dest)?;
                if !is_end(position) {
                    dest.write_str(", ")?;
                    position.to_css(dest)?;
                }
                dest.write_char(')')
            },
            Self::LinearFunction(stops) => {
                dest.write_str("linear(")?;
                stops.to_css(dest)?;
                dest.write_char(')')
            },
        }
    }
}

#[allow(missing_docs)]
#[derive(
    Clone,
    Copy,
    Debug,
    Eq,
    MallocSizeOf,
    Parse,
    PartialEq,
    SpecifiedValueInfo,
    ToComputedValue,
    ToCss,
    ToResolvedValue,
    ToShmem,
    ToTyped,
    Serialize,
    Deserialize,
)]
#[repr(u8)]
pub enum TimingKeyword {
    Linear,
    Ease,
    EaseIn,
    EaseOut,
    EaseInOut,
}

/// Before flag, defined as per https://drafts.csswg.org/css-easing/#before-flag
/// This flag is never user-specified.
#[allow(missing_docs)]
#[derive(PartialEq)]
#[repr(u8)]
pub enum BeforeFlag {
    Unset,
    Set,
}

#[allow(missing_docs)]
#[derive(
    Clone,
    Copy,
    Debug,
    Eq,
    MallocSizeOf,
    Parse,
    PartialEq,
    ToComputedValue,
    ToCss,
    ToResolvedValue,
    ToShmem,
    Serialize,
    Deserialize,
)]
#[repr(u8)]
pub enum StepPosition {
    JumpStart,
    JumpEnd,
    JumpNone,
    JumpBoth,
    Start,
    End,
}

#[inline]
fn is_end(position: &StepPosition) -> bool {
    *position == StepPosition::JumpEnd || *position == StepPosition::End
}

impl<Integer, Number, LinearStops> TimingFunction<Integer, Number, LinearStops> {
    /// `ease`
    #[inline]
    pub fn ease() -> Self {
        TimingFunction::Keyword(TimingKeyword::Ease)
    }

    /// Returns true if it is `ease`.
    #[inline]
    pub fn is_ease(&self) -> bool {
        matches!(*self, TimingFunction::Keyword(TimingKeyword::Ease))
    }
}
