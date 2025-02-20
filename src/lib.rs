//! XML stream reader
//! Like https://gnome.pages.gitlab.gnome.org/libxml2/devhelp/libxml2-xmlreader.html
//! Like https://www.javadoc.io/static/org.codehaus.woodstox/stax2-api/4.2.1/org/codehaus/stax2/XMLStreamReader2.html
//! Like https://learn.microsoft.com/en-us/dotnet/api/system.xml.xmltextreader?view=net-7.0
#![warn(missing_docs)]

use std::ops::{Deref, DerefMut};
use std::vec::Vec;
use xmlparser::{self, ElementEnd, Tokenizer};
pub use xmlparser::{TextPos, Token};

type Result<T> = std::result::Result<T, Error>;

/// A list of all possible errors.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Error {
    /// FIXME
    Unexpected(Option<TextPos>),
    /// Errors detected by the `xmlparser` crate.
    ParserError(xmlparser::Error),
}

impl From<xmlparser::Error> for Error {
    #[inline]
    fn from(e: xmlparser::Error) -> Self {
        Error::ParserError(e)
    }
}

impl std::error::Error for Error {
    #[inline]
    fn description(&self) -> &str {
        "an XML parsing error"
    }
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match *self {
            Error::Unexpected(Some(pos)) => {
                write!(f, "an unexpected error at {}", pos)
            }
            Error::Unexpected(None) => {
                write!(f, "an unexpected error")
            }
            Error::ParserError(ref err) => {
                write!(f, "{}", err)
            }
        }
    }
}

/// XML stream reader
pub struct StreamReader<'input> {
    s: &'input str,
    r: Tokenizer<'input>,
    t: Option<Token<'input>>,
    attrs: Vec<Token<'input>>,
    depth: usize,
}

impl<'input> From<&'input str> for StreamReader<'input> {
    #[inline]
    fn from(text: &'input str) -> Self {
        StreamReader {
            s: text,
            r: Tokenizer::from(text),
            t: None,
            attrs: Vec::new(),
            depth: 0,
        }
    }
}

impl<'input> StreamReader<'input> {
    fn is_start_element(&self) -> bool {
        matches!(
            self.t,
            Some(
                Token::ElementStart { .. }
                    | Token::Attribute { .. }
                    | Token::ElementEnd {
                        end: ElementEnd::Open,
                        ..
                    },
            )
        )
    }

    fn is_empty_token(&self) -> bool {
        matches!(
            self.t,
            Some(Token::ElementEnd {
                end: ElementEnd::Empty,
                ..
            })
        )
    }

    fn fill_attrs(&mut self) -> Result<()> {
        match self.t {
            Some(Token::ElementStart { .. }) => {
                while let Some(t) = self.next_token()? {
                    match t {
                        Token::Attribute { .. } => self.attrs.push(t),
                        _ => {
                            self.t = Some(t);
                            break;
                        }
                    }
                }
                Ok(())
            }
            Some(Token::ElementEnd {
                end: ElementEnd::Open | ElementEnd::Empty,
                ..
            }) => Ok(()),
            _ => {
                Err(Error::Unexpected(self.text_pos_at(&self.t))) // FIXME create specific error
            }
        }
    }

    fn next_token(&mut self) -> Result<Option<Token<'input>>> {
        let t = self.r.next().transpose()?;
        match t {
            Some(Token::ElementEnd {
                end: ElementEnd::Open,
                ..
            }) => self.depth += 1,
            Some(Token::ElementEnd {
                end: ElementEnd::Close(..),
                ..
            }) => self.depth -= 1,
            _ => {}
        };
        Ok(t)
    }

    fn text_pos_at(&self, token: &Option<Token>) -> Option<TextPos> {
        if let Some(ref token) = token {
            let span = match token {
                Token::Declaration { span, .. } => span,
                Token::ProcessingInstruction { span, .. } => span,
                Token::Comment { span, .. } => span,
                Token::DtdStart { span, .. } => span,
                Token::EmptyDtd { span, .. } => span,
                Token::EntityDeclaration { span, .. } => span,
                Token::DtdEnd { span, .. } => span,
                Token::ElementStart { span, .. } => span,
                Token::Attribute { span, .. } => span,
                Token::ElementEnd { span, .. } => span,
                Token::Text { text, .. } => text,
                Token::Cdata { span, .. } => span,
            };
            Some(xmlparser::Stream::from(self.s).gen_text_pos_from(span.start()))
        } else {
            None
        }
    }
}

impl StreamReader<'_> {
    /// number of attributes of the current element
    pub fn attribute_count(&mut self) -> Result<usize> {
        self.fill_attrs()?;
        Ok(self.attrs.len())
    }

    /// name of `i`th attribute
    pub fn attribute_name(&mut self, i: usize) -> Result<Option<&str>> {
        self.fill_attrs()?;
        Ok(self.attrs.get(i).and_then(|t| match t {
            Token::Attribute { local, .. } => Some(local.as_str()),
            _ => None,
        }))
    }

    /// value of `i`th attribute
    pub fn attribute_value(&mut self, i: usize) -> Result<Option<&str>> {
        self.fill_attrs()?;
        Ok(self.attrs.get(i).and_then(|t| match t {
            Token::Attribute { value, .. } => Some(value.as_str()),
            _ => None,
        }))
    }

    /// value of attribute named `name` (local name)
    pub fn attribute(&mut self, name: &str) -> Result<Option<&str>> {
        self.fill_attrs()?;
        Ok(self.attrs.iter().find_map(|t| match t {
            Token::Attribute { local, value, .. } if local.as_str() == name => Some(value.as_str()),
            _ => None,
        }))
    }

    /// depth of the node in the tree.
    // https://gnome.pages.gitlab.gnome.org/libxml2/devhelp/libxml2-xmlreader.html#xmlTextReaderDepth
    pub fn depth(&self) -> usize {
        self.depth
    }

    /// read the content of a text-only element,
    /// an error is thrown if this is not a text-only element.
    // https://gnome.pages.gitlab.gnome.org/libxml2/devhelp/libxml2-xmlreader.html#xmlTextReaderReadString
    // https://github.com/FasterXML/aalto-xml/blob/0820590fcf56ec3d5ca14608d6145e14e56f2650/src/main/java/com/fasterxml/aalto/stax/StreamReaderImpl.java#L403
    pub fn element_text(&mut self) -> Result<Option<&str>> {
        if !self.is_start_element() {
            if self.is_empty_token() {
                return Ok(None);
            }
            Err(Error::Unexpected(self.text_pos_at(&self.t))) // FIXME create specific error
        } else {
            let mut txt = None;
            while self.next()?.is_some() {
                match self.t {
                    // TODO cumulate text mixed with comments / pi
                    Some(Token::Text { text, .. } | Token::Cdata { text, .. }) => {
                        if txt.is_none() {
                            txt = Some(text.as_str());
                        } else {
                            return Err(Error::Unexpected(self.text_pos_at(&self.t)));
                        }
                    }
                    Some(Token::Comment { .. } | Token::ProcessingInstruction { .. }) => continue,
                    Some(Token::ElementEnd { end, .. }) => match end {
                        ElementEnd::Open => continue,
                        ElementEnd::Empty => break,
                        ElementEnd::Close(..) => {
                            if txt.is_none() {
                                txt = Some("")
                            }
                            break;
                        }
                    },
                    _ => return Err(Error::Unexpected(self.text_pos_at(&self.t))),
                }
            }
            Ok(txt)
        }
    }

    //fn event_type(&self) ->
    /// `true` if the current token has a name
    pub fn has_name(&self) -> bool {
        matches!(
            self.t,
            Some(
                Token::ElementStart { .. }
                    | Token::ElementEnd {
                        end: ElementEnd::Close(..),
                        ..
                    },
            )
        )
    }

    /// return the (local) name of the current token,
    /// an error is thrown if this is not a named element.
    // https://gnome.pages.gitlab.gnome.org/libxml2/devhelp/libxml2-xmlreader.html#xmlTextReaderLocalName
    pub fn local_name(&self) -> Result<&str> {
        match self.t {
            Some(
                Token::ElementStart { local, .. }
                | Token::ElementEnd {
                    end: ElementEnd::Close(_, local),
                    ..
                },
            ) => Ok(local.as_str()),
            _ => Err(Error::Unexpected(self.text_pos_at(&self.t))), // FIXME create specific error
        }
    }

    /// element ending with "/>"
    // https://gnome.pages.gitlab.gnome.org/libxml2/devhelp/libxml2-xmlreader.html#xmlTextReaderIsEmptyElement
    pub fn is_empty_element(&mut self) -> Result<bool> {
        if !self.is_start_element() {
            if self.is_empty_token() {
                return Ok(true);
            }
            return Ok(false);
        }
        while let Some(t) = self.next()? {
            match t {
                Token::Attribute { .. } => continue,
                Token::ElementEnd {
                    end: ElementEnd::Empty,
                    ..
                } => return Ok(true),
                _ => break,
            }
        }
        Ok(false)
    }

    //fn has_next() -> bool
    /// get next token
    pub fn next(&mut self) -> Result<Option<Token>> {
        self.t = self.next_token()?;
        if let Some(Token::ElementStart { .. }) = self.t {
            self.attrs.clear();
        }
        Ok(self.t)
    }

    /// go to next tag
    pub fn next_tag(&mut self) -> Result<Option<Token>> {
        self.next()?;
        while !matches!(self.t, Some(Token::ElementStart { .. }) | None) {
            self.next()?;
        }
        Ok(self.t)
    }

    /// skip all the contents of the current element
    pub fn skip_element(&mut self) -> Result<()> {
        if !self.is_start_element() {
            return Err(Error::Unexpected(self.text_pos_at(&self.t))); // FIXME create specific error
        }
        let depth = self.depth;
        while let Some(t) = self.next_token()? {
            if self.depth == depth {
                if let Token::ElementEnd {
                    end: ElementEnd::Empty | ElementEnd::Close(..),
                    ..
                } = t
                {
                    self.t = Some(t);
                    break;
                }
            }
        }
        Ok(())
    }

    /// `true` if the current token has text
    pub fn has_text(&self) -> bool {
        matches!(
            self.t,
            Some(Token::Text { .. } | Token::Cdata { .. } | Token::Comment { .. })
        )
    }

    /// return the current token's string,
    /// an error is thrown if this kind of token has no text.
    pub fn text(&self) -> Result<&str> {
        match self.t {
            Some(
                Token::Text { text, .. } | Token::Cdata { text, .. } | Token::Comment { text, .. },
            ) => Ok(text.as_str()),
            _ => Err(Error::Unexpected(self.text_pos_at(&self.t))), // FIXME create specific error
        }
    }
}

/// Sub-tree reader
pub struct SubTreeReader<'input, 'l> {
    sr: &'l mut StreamReader<'input>,
    initial_depth: usize,
    eos: bool,
}

impl<'input, 'l> SubTreeReader<'input, 'l> {
    /// constructor
    pub fn new(sr: &'l mut StreamReader<'input>) -> Result<SubTreeReader<'input, 'l>> {
        let initial_depth = if matches!(
            sr.t,
            Some(Token::ElementStart { .. } | Token::Attribute { .. })
        ) {
            sr.depth()
        } else if matches!(
            sr.t,
            Some(Token::ElementEnd {
                end: ElementEnd::Open,
                ..
            })
        ) {
            sr.depth() - 1
        } else {
            return Err(Error::Unexpected(sr.text_pos_at(&sr.t))); // FIXME create specific error
        };
        Ok(SubTreeReader {
            sr,
            initial_depth,
            eos: false,
        })
    }

    /// get next token
    pub fn next(&mut self) -> Result<Option<Token>> {
        if self.is_eos() {
            return Ok(None);
        }
        self.sr.next()?;
        Ok(self.sr.t)
    }

    /// go to next tag
    pub fn next_tag(&mut self) -> Result<Option<Token>> {
        self.next()?;
        while !self.eos && !matches!(self.sr.t, Some(Token::ElementStart { .. }) | None) {
            self.next()?;
        }
        Ok(if self.eos { None } else { self.sr.t })
    }

    fn is_eos(&mut self) -> bool {
        if self.eos {
            return true;
        }
        if let Some(t) = self.sr.t {
            if self.sr.depth == self.initial_depth {
                if let Token::ElementEnd {
                    end: ElementEnd::Empty | ElementEnd::Close(..),
                    ..
                } = t
                {
                    self.eos = true;
                    return true;
                }
            }
        }
        false
    }
}

impl<'input> Deref for SubTreeReader<'input, '_> {
    type Target = StreamReader<'input>;

    #[inline]
    fn deref(&self) -> &StreamReader<'input> {
        self.sr
    }
}
impl<'input> DerefMut for SubTreeReader<'input, '_> {
    #[inline]
    fn deref_mut(&mut self) -> &mut StreamReader<'input> {
        self.sr
    }
}

#[cfg(test)]
mod test {
    use super::StreamReader;
    use crate::Result;

    #[test]
    fn attrs() -> Result<()> {
        let mut sr = StreamReader::from("<root/>");
        assert!(sr.attribute_count().is_err());
        assert!(sr.attribute_name(0).is_err());
        assert!(sr.attribute_value(0).is_err());
        assert!(sr.attribute("id").is_err());
        assert!(sr.next()?.is_some());
        assert_eq!(sr.attribute_count()?, 0);
        assert!(sr.attribute_name(0)?.is_none());
        assert!(sr.attribute_value(0)?.is_none());
        assert!(sr.attribute("id")?.is_none());
        let mut sr = StreamReader::from("<root id='1' value='x'/>");
        assert!(sr.next()?.is_some());
        assert_eq!(sr.attribute_count()?, 2);
        assert_eq!(sr.attribute_name(0)?, Some("id"));
        assert_eq!(sr.attribute_value(0)?, Some("1"));
        assert_eq!(sr.attribute("id")?, Some("1"));
        assert_eq!(sr.attribute_name(1)?, Some("value"));
        assert_eq!(sr.attribute_value(1)?, Some("x"));
        assert_eq!(sr.attribute("value")?, Some("x"));
        Ok(())
    }

    #[test]
    fn element_text() -> Result<()> {
        let mut sr = StreamReader::from("<root/>");
        assert!(sr.element_text().is_err());
        assert!(sr.next()?.is_some());
        assert!(sr.element_text()?.is_none());
        let mut sr = StreamReader::from("<root></root>");
        assert!(sr.next()?.is_some());
        assert_eq!(sr.element_text()?, Some(""));
        let mut sr = StreamReader::from("<root>data</root>");
        assert!(sr.next()?.is_some());
        assert_eq!(sr.element_text()?, Some("data"));
        let mut sr = StreamReader::from("<root><child/>data</root>");
        assert!(sr.next()?.is_some());
        assert!(sr.element_text().is_err());
        let mut sr = StreamReader::from("<root>data<child/></root>");
        assert!(sr.next()?.is_some());
        assert!(sr.element_text().is_err());
        Ok(())
    }

    #[test]
    fn has_name() -> Result<()> {
        let mut sr = StreamReader::from("<root/>");
        assert!(!sr.has_name());
        assert!(sr.next()?.is_some());
        assert!(sr.has_name());
        assert!(sr.next()?.is_some());
        assert!(!sr.has_name()); // FIXME
        Ok(())
    }

    #[ignore] // FIXME
    #[test]
    fn local_name() -> Result<()> {
        let mut sr = StreamReader::from("<root/>");
        assert!(sr.local_name().is_err());
        while sr.next()?.is_some() {
            assert_eq!(sr.local_name()?, "root");
        }
        Ok(())
    }

    #[test]
    fn is_empty_element() -> Result<()> {
        let mut sr = StreamReader::from("<root/>");
        assert!(!sr.is_empty_element()?);
        assert!(sr.next()?.is_some());
        assert!(sr.is_empty_element()?);
        let mut sr = StreamReader::from("<root></root>");
        assert!(sr.next()?.is_some());
        assert!(!sr.is_empty_element()?);
        Ok(())
    }

    #[test]
    fn skip_element() -> Result<()> {
        let mut sr = StreamReader::from("<root/>");
        assert!(sr.skip_element().is_err());
        assert!(sr.next()?.is_some());
        sr.skip_element()?;
        assert!(sr.is_empty_token());
        let mut sr = StreamReader::from("<root><child/><child></child></root>");
        sr.next()?;
        sr.skip_element()?;
        assert_eq!(sr.depth(), 0);
        assert_eq!(sr.local_name()?, "root");
        assert!(sr.next()?.is_none());
        let mut sr = StreamReader::from("<a><a/></a>");
        sr.next()?;
        sr.skip_element()?;
        assert!(sr.next()?.is_none());
        Ok(())
    }

    #[test]
    fn text() -> Result<()> {
        let mut sr = StreamReader::from("<root/>");
        assert!(!sr.has_text());
        while sr.next()?.is_some() {
            assert!(sr.text().is_err());
        }
        let mut sr = StreamReader::from("<root>data</root>");
        while !sr.has_text() {
            sr.next()?;
        }
        assert_eq!(sr.text()?, "data");
        Ok(())
    }
}
