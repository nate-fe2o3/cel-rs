use std::sync::LazyLock;

use proc_macro2::{Delimiter, Group, Ident, Literal, Punct, Spacing, Span, TokenStream, TokenTree};

type LS = LazyLock<TokenStream>;
type LT = LazyLock<TokenTree>;

//Joint
pub const OR_J: LT = LazyLock::new(|| TokenTree::Punct(Punct::new('|', Spacing::Joint)));
pub const AND_J: LT = LazyLock::new(|| TokenTree::Punct(Punct::new('&', Spacing::Joint)));
pub const EQ_J: LT = LazyLock::new(|| TokenTree::Punct(Punct::new('=', Spacing::Joint)));
pub const NOT_J: LT = LazyLock::new(|| TokenTree::Punct(Punct::new('!', Spacing::Joint)));
pub const LT_J: LT = LazyLock::new(|| TokenTree::Punct(Punct::new('<', Spacing::Joint)));
pub const GT_J: LT = LazyLock::new(|| TokenTree::Punct(Punct::new('>', Spacing::Joint)));

//Trees
pub const QUESTION: LT = LazyLock::new(|| TokenTree::Punct(Punct::new('?', Spacing::Alone)));
pub const DOT: LT = LazyLock::new(|| TokenTree::Punct(Punct::new('.', Spacing::Alone)));
pub const COMMA: LT = LazyLock::new(|| TokenTree::Punct(Punct::new(',', Spacing::Alone)));
pub const COLON: LT = LazyLock::new(|| TokenTree::Punct(Punct::new(':', Spacing::Alone)));
pub const BITOR: LT = LazyLock::new(|| TokenTree::Punct(Punct::new('|', Spacing::Alone)));
pub const BITXOR: LT = LazyLock::new(|| TokenTree::Punct(Punct::new('^', Spacing::Alone)));
pub const BITAND: LT = LazyLock::new(|| TokenTree::Punct(Punct::new('&', Spacing::Alone)));
pub const ASSIGN: LT = LazyLock::new(|| TokenTree::Punct(Punct::new('=', Spacing::Alone)));
pub const LT: LT = LazyLock::new(|| TokenTree::Punct(Punct::new('<', Spacing::Alone)));
pub const GT: LT = LazyLock::new(|| TokenTree::Punct(Punct::new('>', Spacing::Alone)));
pub const ADD: LT = LazyLock::new(|| TokenTree::Punct(Punct::new('+', Spacing::Alone)));
pub const SUB: LT = LazyLock::new(|| TokenTree::Punct(Punct::new('-', Spacing::Alone)));
pub const HIGHMINUS: LT = LazyLock::new(|| TokenTree::Punct(Punct::new('-', Spacing::Joint)));
pub const MUL: LT = LazyLock::new(|| TokenTree::Punct(Punct::new('*', Spacing::Alone)));
pub const DIV: LT = LazyLock::new(|| TokenTree::Punct(Punct::new('/', Spacing::Alone)));
pub const MOD: LT = LazyLock::new(|| TokenTree::Punct(Punct::new('%', Spacing::Alone)));
pub const NOT: LT = LazyLock::new(|| TokenTree::Punct(Punct::new('!', Spacing::Alone)));
pub const AT: LT = LazyLock::new(|| TokenTree::Punct(Punct::new('@', Spacing::Alone)));
pub const SQUARE_GROUP: LT =
    LazyLock::new(|| TokenTree::Group(Group::new(Delimiter::Bracket, TokenStream::new())));
pub const PARENS_GROUP: LT =
    LazyLock::new(|| TokenTree::Group(Group::new(Delimiter::Parenthesis, TokenStream::new())));
pub const CURLY_GROUP: LT =
    LazyLock::new(|| TokenTree::Group(Group::new(Delimiter::Brace, TokenStream::new())));
pub const IDENT: LT = LazyLock::new(|| TokenTree::Ident(Ident::new("testing", Span::call_site())));
pub const LIT: LT = LazyLock::new(|| TokenTree::Literal(Literal::string("any literal")));
