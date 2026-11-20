//! Moli-facing font capabilities and `@font-face` parsing hooks.

use std::borrow::Cow;

use cssparser::{
    AtRuleParser, CowRcStr, Parser, QualifiedRuleParser, SourceLocation, StyleSheetParser,
};
use style_traits::{CssWriter, ParsingMode, ToCss};

use crate::{
    context::QuirksMode,
    custom_properties::AttrTaint,
    font_face::{
        FontFaceSourceFormat, FontFaceSourceFormatKeyword, FontFaceSourceTechFlags, SourceList,
        UrlSource, parse_font_face_block,
    },
    parser::{Parse, ParserContext},
    stylesheets::{CssRuleType, Namespaces, Origin, UrlExtraData},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CssFontFace {
    pub family: String,
    pub source: String,
}

pub fn parse_font_faces(css_text: &str) -> Vec<CssFontFace> {
    with_font_face_context(|context| {
        let input = css_text;
        let mut parser_input = Parser::new(input);
        let mut parser = FontFaceRuleParser { context };
        StyleSheetParser::new(&mut parser_input, &mut parser)
            .filter_map(Result::ok)
            .collect()
    })
    .unwrap_or_default()
}

pub fn normalize_font_face_src(value: &str) -> Option<String> {
    to_css_string(&parse_font_face_source_list(value)?)
}

/// Parse a source list for which the caller resolves relative URLs.
pub fn parse_font_face_source_list(value: &str) -> Option<SourceList> {
    with_font_face_context(|context| {
        Parser::new(value)
            .parse_entirely(|input| SourceList::parse(context, input))
            .ok()
    })?
}

/// Font formats supported by Moli's font loader.
pub fn supports_font_format(format: FontFaceSourceFormatKeyword) -> bool {
    use FontFaceSourceFormatKeyword::*;
    matches!(format, Collection | Opentype | Truetype | Woff | Woff2)
}

/// Whether Moli can use a font requiring all the specified technologies.
pub fn supports_font_tech(tech: FontFaceSourceTechFlags) -> bool {
    use FontFaceSourceTechFlags as Tech;
    let supported = Tech::FEATURES_OPENTYPE
        | Tech::FEATURES_AAT
        | Tech::COLOR_COLRV0
        | Tech::COLOR_COLRV1
        | Tech::COLOR_SBIX
        | Tech::COLOR_CBDT
        | Tech::VARIATIONS;
    supported.contains(tech)
}

/// Whether Moli can try loading a URL with these format and technology hints.
pub fn supports_font_source(source: &UrlSource) -> bool {
    supports_font_tech(source.tech_flags)
        && source.format_hint.as_ref().is_none_or(|hint| match hint {
            FontFaceSourceFormat::Keyword(format) => supports_font_format(*format),
            FontFaceSourceFormat::String(format) => supports_legacy_format_hint(format),
        })
}

fn supports_legacy_format_hint(format: &str) -> bool {
    if !format
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    {
        return false;
    }
    let format = format.to_ascii_lowercase();
    let format = match format.as_str() {
        "otf" | "opentype-variations" => "opentype",
        "ttf" | "truetype-variations" => "truetype",
        "woff-variations" => "woff",
        "woff2-variations" => "woff2",
        format => format,
    };
    Parser::new(format)
        .parse_entirely(FontFaceSourceFormatKeyword::parse)
        .is_ok_and(supports_font_format)
}

fn with_font_face_context<R>(f: impl FnOnce(&ParserContext) -> R) -> Option<R> {
    let url_data = UrlExtraData::from(url::Url::parse("about:blank").ok()?);
    let context = ParserContext::new(
        Origin::Author,
        &url_data,
        Some(CssRuleType::FontFace),
        ParsingMode::DEFAULT,
        QuirksMode::NoQuirks,
        Cow::Owned(Namespaces::default()),
        None,
        None,
        AttrTaint::default(),
    );
    Some(f(&context))
}

struct FontFaceRuleParser<'a> {
    context: &'a ParserContext<'a>,
}

impl<'a, 'i> AtRuleParser<'i> for FontFaceRuleParser<'a> {
    type Prelude = SourceLocation;
    type AtRule = CssFontFace;
    type Error = ();

    fn parse_prelude<'t>(
        &mut self,
        name: CowRcStr<'i>,
        input: &mut Parser,
    ) -> Result<Self::Prelude, cssparser::ParseError<Self::Error>> {
        if !name.eq_ignore_ascii_case("font-face") {
            return Err(cssparser::ParseError::custom(()));
        }
        let source_location = input.current_source_location();
        input.expect_exhausted()?;
        Ok(source_location)
    }

    fn parse_block<'t>(
        &mut self,
        source_location: Self::Prelude,
        _start: &cssparser::ParserState,
        input: &mut Parser,
    ) -> Result<Self::AtRule, cssparser::ParseError<Self::Error>> {
        let rule = parse_font_face_block(self.context, input, source_location);
        let family = rule
            .descriptors
            .font_family
            .ok_or_else(|| cssparser::ParseError::custom(()))?;
        let source = rule
            .descriptors
            .src
            .as_ref()
            .and_then(to_css_string)
            .ok_or_else(|| cssparser::ParseError::custom(()))?;
        Ok(CssFontFace {
            family: family.name.to_string(),
            source,
        })
    }
}

impl<'a, 'i> QualifiedRuleParser<'i> for FontFaceRuleParser<'a> {
    type Prelude = ();
    type QualifiedRule = CssFontFace;
    type Error = ();
}

fn to_css_string<T: ToCss>(value: &T) -> Option<String> {
    let mut output = String::new();
    value.to_css(&mut CssWriter::new(&mut output)).ok()?;
    Some(output)
}

#[cfg(test)]
mod tests {
    use super::{normalize_font_face_src, parse_font_faces};

    #[test]
    fn font_face_parser_uses_stylo_rule_and_descriptor_boundaries() {
        let entries = parse_font_faces(
            r#"
            .ignored { content: "@font-face { font-family: Bad; src: url(bad.woff2); }"; }
            @font-face {
                font-family: "A; B";
                src: url("data:font/woff2;base64;a;b");
            }
            @FONT-FACE {
                font-family: CaseFace;
                src: local("Case Face");
            }
            "#,
        );

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].family, "A; B");
        assert_eq!(entries[0].source, r#"url("data:font/woff2;base64;a;b")"#);
        assert_eq!(entries[1].family, "CaseFace");
        assert_eq!(entries[1].source, r#"local("Case Face")"#);
    }

    #[test]
    fn font_face_parser_filters_invalid_and_incomplete_faces() {
        let entries = parse_font_faces(
            r#"
            @font-face { font-family: serif; src: url(generic.woff2); }
            @font-face { font-family: MissingSource; }
            @font-face { src: url(missing-family.woff2); }
            @font-face { font-family: Valid; src: url(valid.woff2); }
            "#,
        );

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].family, "Valid");
        assert_eq!(entries[0].source, r#"url("valid.woff2")"#);
    }

    #[test]
    fn font_face_src_normalizer_uses_stylo_source_list() {
        assert_eq!(
            normalize_font_face_src("local(STIXGeneral), url(/stixfonts/STIXGeneral.otf)")
                .as_deref(),
            Some(r#"local(STIXGeneral), url("/stixfonts/STIXGeneral.otf")"#)
        );
        assert_eq!(
            normalize_font_face_src("url(http://foo/bar/font.ttf)").as_deref(),
            Some(r#"url("http://foo/bar/font.ttf")"#)
        );
    }
}
