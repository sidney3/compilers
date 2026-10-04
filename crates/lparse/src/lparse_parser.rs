#![allow(clippy::all)]
use crate::lparse_frontend::BoundLeaf;
use crate::lparse_frontend::BoundRule;
use crate::lparse_frontend::Ident;
use crate::lparse_frontend::LGrammar;
use crate::lparse_frontend::LNode;
use crate::lparse_frontend::LParseToken;
use crate::lparse_frontend::LRule;
use crate::lparse_frontend::ProductionDefinition;
use lasso::Rodeo;
use lexer::Tokens;
use lox_derive::Ordinal;
use strum::Display;
use lparse;
#[derive(Ordinal, Eq, PartialEq, Hash, Display, Debug, PartialOrd)]
enum ParseRule {
    LGrammar,
    BoundLeaf,
    BoundRule,
    BoundKleeneRule,
    LNode,
    ProductionDefinition,
    LRule,
    SetGoalRule,
    SetTokenType,
    Preamble,
    ReservedLNodeKleene,
    ReservedProductionDefinitionKleene,
    ReservedPreambleKleene,
    ReservedLRuleKleene,
}
impl lparse::Rule for ParseRule {
    type TokenType = LParseToken;
}
pub struct LParseParser {
    parser: lparse::Parser<ParseRule>,
}
impl LParseParser {
    pub fn new() -> Self {
        Self {
            parser: lparse::Parser::new(__make_grammar()),
        }
    }
    pub fn parse(
        &self,
        tokens: Tokens<LParseToken>,
    ) -> Result<(Rodeo, LGrammar), lparse::Error> {
        let cst = self.parser.parse(tokens)?;
        if let lparse::Node::Parent(root) = &cst.root {
            Ok((cst.lexeme_arena, __rule_factory_function_l_grammar(&root)))
        } else {
            panic!("Unreachable, root of CST is a token");
        }
    }
}
fn __make_grammar() -> lparse::Grammar<ParseRule> {
    lparse::Grammar::new(
        ParseRule::LGrammar,
        Vec::from([
            lparse::Production::<ParseRule> {
                rule: ParseRule::LGrammar,
                definition: Vec::from([
                    lparse::Symbol::Rule(ParseRule::ReservedPreambleKleene),
                    lparse::Symbol::Rule(ParseRule::SetGoalRule),
                    lparse::Symbol::Rule(ParseRule::SetTokenType),
                    lparse::Symbol::Rule(ParseRule::ReservedLRuleKleene),
                ]),
            },
            lparse::Production::<ParseRule> {
                rule: ParseRule::BoundLeaf,
                definition: Vec::from([
                    lparse::Symbol::Token(LParseToken::LSquareBracket),
                    lparse::Symbol::Token(LParseToken::Ident),
                    lparse::Symbol::Token(LParseToken::Colon),
                    lparse::Symbol::Token(LParseToken::Ident),
                    lparse::Symbol::Token(LParseToken::RSquareBracket),
                ]),
            },
            lparse::Production::<ParseRule> {
                rule: ParseRule::BoundRule,
                definition: Vec::from([
                    lparse::Symbol::Token(LParseToken::LAngleBracket),
                    lparse::Symbol::Token(LParseToken::Ident),
                    lparse::Symbol::Token(LParseToken::Colon),
                    lparse::Symbol::Token(LParseToken::Ident),
                    lparse::Symbol::Token(LParseToken::RAngleBracket),
                ]),
            },
            lparse::Production::<ParseRule> {
                rule: ParseRule::BoundKleeneRule,
                definition: Vec::from([
                    lparse::Symbol::Token(LParseToken::LAngleBracket),
                    lparse::Symbol::Token(LParseToken::Ident),
                    lparse::Symbol::Token(LParseToken::Colon),
                    lparse::Symbol::Token(LParseToken::Ident),
                    lparse::Symbol::Token(LParseToken::Asterisk),
                    lparse::Symbol::Token(LParseToken::RAngleBracket),
                ]),
            },
            lparse::Production::<ParseRule> {
                rule: ParseRule::LNode,
                definition: Vec::from([lparse::Symbol::Rule(ParseRule::BoundLeaf)]),
            },
            lparse::Production::<ParseRule> {
                rule: ParseRule::LNode,
                definition: Vec::from([lparse::Symbol::Rule(ParseRule::BoundRule)]),
            },
            lparse::Production::<ParseRule> {
                rule: ParseRule::LNode,
                definition: Vec::from([lparse::Symbol::Rule(ParseRule::BoundKleeneRule)]),
            },
            lparse::Production::<ParseRule> {
                rule: ParseRule::ProductionDefinition,
                definition: Vec::from([
                    lparse::Symbol::Rule(ParseRule::ReservedLNodeKleene),
                    lparse::Symbol::Token(LParseToken::Equals),
                    lparse::Symbol::Token(LParseToken::RAngleBracket),
                    lparse::Symbol::Token(LParseToken::EmbeddedRust),
                    lparse::Symbol::Token(LParseToken::Comma),
                ]),
            },
            lparse::Production::<ParseRule> {
                rule: ParseRule::LRule,
                definition: Vec::from([
                    lparse::Symbol::Token(LParseToken::Ident),
                    lparse::Symbol::Token(LParseToken::Colon),
                    lparse::Symbol::Token(LParseToken::EmbeddedRust),
                    lparse::Symbol::Token(LParseToken::LCurlyBrace),
                    lparse::Symbol::Rule(ParseRule::ReservedProductionDefinitionKleene),
                    lparse::Symbol::Token(LParseToken::RCurlyBrace),
                    lparse::Symbol::Token(LParseToken::Semicolon),
                ]),
            },
            lparse::Production::<ParseRule> {
                rule: ParseRule::LRule,
                definition: Vec::from([
                    lparse::Symbol::Token(LParseToken::Ident),
                    lparse::Symbol::Token(LParseToken::LCurlyBrace),
                    lparse::Symbol::Rule(ParseRule::ReservedProductionDefinitionKleene),
                    lparse::Symbol::Token(LParseToken::RCurlyBrace),
                    lparse::Symbol::Token(LParseToken::Semicolon),
                ]),
            },
            lparse::Production::<ParseRule> {
                rule: ParseRule::SetGoalRule,
                definition: Vec::from([
                    lparse::Symbol::Token(LParseToken::GoalRule),
                    lparse::Symbol::Token(LParseToken::Equals),
                    lparse::Symbol::Token(LParseToken::Ident),
                    lparse::Symbol::Token(LParseToken::Semicolon),
                ]),
            },
            lparse::Production::<ParseRule> {
                rule: ParseRule::SetTokenType,
                definition: Vec::from([
                    lparse::Symbol::Token(LParseToken::TokenType),
                    lparse::Symbol::Token(LParseToken::Equals),
                    lparse::Symbol::Token(LParseToken::Ident),
                    lparse::Symbol::Token(LParseToken::Semicolon),
                ]),
            },
            lparse::Production::<ParseRule> {
                rule: ParseRule::Preamble,
                definition: Vec::from([lparse::Symbol::Token(LParseToken::RustImport)]),
            },
            lparse::Production::<ParseRule> {
                rule: ParseRule::Preamble,
                definition: Vec::from([
                    lparse::Symbol::Token(LParseToken::RustDirective),
                ]),
            },
            lparse::Production::<ParseRule> {
                rule: ParseRule::ReservedLNodeKleene,
                definition: Vec::from([]),
            },
            lparse::Production::<ParseRule> {
                rule: ParseRule::ReservedLNodeKleene,
                definition: Vec::from([
                    lparse::Symbol::Rule(ParseRule::ReservedLNodeKleene),
                    lparse::Symbol::Rule(ParseRule::LNode),
                ]),
            },
            lparse::Production::<ParseRule> {
                rule: ParseRule::ReservedProductionDefinitionKleene,
                definition: Vec::from([]),
            },
            lparse::Production::<ParseRule> {
                rule: ParseRule::ReservedProductionDefinitionKleene,
                definition: Vec::from([
                    lparse::Symbol::Rule(ParseRule::ReservedProductionDefinitionKleene),
                    lparse::Symbol::Rule(ParseRule::ProductionDefinition),
                ]),
            },
            lparse::Production::<ParseRule> {
                rule: ParseRule::ReservedPreambleKleene,
                definition: Vec::from([]),
            },
            lparse::Production::<ParseRule> {
                rule: ParseRule::ReservedPreambleKleene,
                definition: Vec::from([
                    lparse::Symbol::Rule(ParseRule::ReservedPreambleKleene),
                    lparse::Symbol::Rule(ParseRule::Preamble),
                ]),
            },
            lparse::Production::<ParseRule> {
                rule: ParseRule::ReservedLRuleKleene,
                definition: Vec::from([]),
            },
            lparse::Production::<ParseRule> {
                rule: ParseRule::ReservedLRuleKleene,
                definition: Vec::from([
                    lparse::Symbol::Rule(ParseRule::ReservedLRuleKleene),
                    lparse::Symbol::Rule(ParseRule::LRule),
                ]),
            },
        ]),
    )
}
fn __rule_factory_function_l_grammar(node: &lparse::Parent<ParseRule>) -> LGrammar {
    match (&node.rule, node.children()) {
        (
            ParseRule::LGrammar,
            [lparse::Node::Parent(__node_0),
            lparse::Node::Parent(__node_1),
            lparse::Node::Parent(__node_2),
            lparse::Node::Parent(__node_3),
            ],
        ) if true && __node_0.rule == ParseRule::ReservedPreambleKleene
            && __node_1.rule == ParseRule::SetGoalRule
            && __node_2.rule == ParseRule::SetTokenType
            && __node_3.rule == ParseRule::ReservedLRuleKleene => {
            let preamble = __rule_factory_function_reserved_preamble_kleene(__node_0);
            let goalrule = __rule_factory_function_set_goal_rule(__node_1);
            let tokentype = __rule_factory_function_set_token_type(__node_2);
            let rules = __rule_factory_function_reserved_l_rule_kleene(__node_3);
            let _span = node.span();
            LGrammar {
                preamble,
                goal_rule: goalrule,
                token_type: tokentype,
                rules,
            }
        }
        _ => panic!("Unreachable"),
    }
}
fn __rule_factory_function_bound_leaf(node: &lparse::Parent<ParseRule>) -> BoundLeaf {
    match (&node.rule, node.children()) {
        (
            ParseRule::BoundLeaf,
            [lparse::Node::Leaf(__node_0),
            lparse::Node::Leaf(__node_1),
            lparse::Node::Leaf(__node_2),
            lparse::Node::Leaf(__node_3),
            lparse::Node::Leaf(__node_4),
            ],
        ) if true && __node_0.token_type == LParseToken::LSquareBracket
            && __node_1.token_type == LParseToken::Ident
            && __node_2.token_type == LParseToken::Colon
            && __node_3.token_type == LParseToken::Ident
            && __node_4.token_type == LParseToken::RSquareBracket => {
            let _ = __node_0.lexeme;
            let bind_to = __node_1.lexeme;
            let _ = __node_2.lexeme;
            let token = __node_3.lexeme;
            let _ = __node_4.lexeme;
            let _span = node.span();
            BoundLeaf {
                token: token,
                bind_to: bind_to,
            }
        }
        _ => panic!("Unreachable"),
    }
}
fn __rule_factory_function_bound_rule(node: &lparse::Parent<ParseRule>) -> BoundRule {
    match (&node.rule, node.children()) {
        (
            ParseRule::BoundRule,
            [lparse::Node::Leaf(__node_0),
            lparse::Node::Leaf(__node_1),
            lparse::Node::Leaf(__node_2),
            lparse::Node::Leaf(__node_3),
            lparse::Node::Leaf(__node_4),
            ],
        ) if true && __node_0.token_type == LParseToken::LAngleBracket
            && __node_1.token_type == LParseToken::Ident
            && __node_2.token_type == LParseToken::Colon
            && __node_3.token_type == LParseToken::Ident
            && __node_4.token_type == LParseToken::RAngleBracket => {
            let _ = __node_0.lexeme;
            let bind_to = __node_1.lexeme;
            let _ = __node_2.lexeme;
            let token = __node_3.lexeme;
            let _ = __node_4.lexeme;
            let _span = node.span();
            BoundRule {
                rule: token,
                bind_to: bind_to,
            }
        }
        _ => panic!("Unreachable"),
    }
}
fn __rule_factory_function_bound_kleene_rule(
    node: &lparse::Parent<ParseRule>,
) -> BoundRule {
    match (&node.rule, node.children()) {
        (
            ParseRule::BoundKleeneRule,
            [lparse::Node::Leaf(__node_0),
            lparse::Node::Leaf(__node_1),
            lparse::Node::Leaf(__node_2),
            lparse::Node::Leaf(__node_3),
            lparse::Node::Leaf(__node_4),
            lparse::Node::Leaf(__node_5),
            ],
        ) if true && __node_0.token_type == LParseToken::LAngleBracket
            && __node_1.token_type == LParseToken::Ident
            && __node_2.token_type == LParseToken::Colon
            && __node_3.token_type == LParseToken::Ident
            && __node_4.token_type == LParseToken::Asterisk
            && __node_5.token_type == LParseToken::RAngleBracket => {
            let _ = __node_0.lexeme;
            let bind_to = __node_1.lexeme;
            let _ = __node_2.lexeme;
            let token = __node_3.lexeme;
            let _ = __node_4.lexeme;
            let _ = __node_5.lexeme;
            let _span = node.span();
            BoundRule { rule: token, bind_to }
        }
        _ => panic!("Unreachable"),
    }
}
fn __rule_factory_function_l_node(node: &lparse::Parent<ParseRule>) -> LNode {
    match (&node.rule, node.children()) {
        (
            ParseRule::LNode,
            [lparse::Node::Parent(__node_0),
            ],
        ) if true && __node_0.rule == ParseRule::BoundLeaf => {
            let leaf = __rule_factory_function_bound_leaf(__node_0);
            let _span = node.span();
            LNode::Leaf(leaf)
        }
        (
            ParseRule::LNode,
            [lparse::Node::Parent(__node_0),
            ],
        ) if true && __node_0.rule == ParseRule::BoundRule => {
            let rule = __rule_factory_function_bound_rule(__node_0);
            let _span = node.span();
            LNode::Rule(rule)
        }
        (
            ParseRule::LNode,
            [lparse::Node::Parent(__node_0),
            ],
        ) if true && __node_0.rule == ParseRule::BoundKleeneRule => {
            let rule = __rule_factory_function_bound_kleene_rule(__node_0);
            let _span = node.span();
            LNode::Kleene(rule, ())
        }
        _ => panic!("Unreachable"),
    }
}
fn __rule_factory_function_production_definition(
    node: &lparse::Parent<ParseRule>,
) -> ProductionDefinition {
    match (&node.rule, node.children()) {
        (
            ParseRule::ProductionDefinition,
            [lparse::Node::Parent(__node_0),
            lparse::Node::Leaf(__node_1),
            lparse::Node::Leaf(__node_2),
            lparse::Node::Leaf(__node_3),
            lparse::Node::Leaf(__node_4),
            ],
        ) if true && __node_0.rule == ParseRule::ReservedLNodeKleene
            && __node_1.token_type == LParseToken::Equals
            && __node_2.token_type == LParseToken::RAngleBracket
            && __node_3.token_type == LParseToken::EmbeddedRust
            && __node_4.token_type == LParseToken::Comma => {
            let definition = __rule_factory_function_reserved_l_node_kleene(__node_0);
            let _ = __node_1.lexeme;
            let _ = __node_2.lexeme;
            let semantic_action = __node_3.lexeme;
            let _ = __node_4.lexeme;
            let _span = node.span();
            ProductionDefinition {
                definition: definition,
                semantic_action: semantic_action,
            }
        }
        _ => panic!("Unreachable"),
    }
}
fn __rule_factory_function_l_rule(node: &lparse::Parent<ParseRule>) -> LRule {
    match (&node.rule, node.children()) {
        (
            ParseRule::LRule,
            [lparse::Node::Leaf(__node_0),
            lparse::Node::Leaf(__node_1),
            lparse::Node::Leaf(__node_2),
            lparse::Node::Leaf(__node_3),
            lparse::Node::Parent(__node_4),
            lparse::Node::Leaf(__node_5),
            lparse::Node::Leaf(__node_6),
            ],
        ) if true && __node_0.token_type == LParseToken::Ident
            && __node_1.token_type == LParseToken::Colon
            && __node_2.token_type == LParseToken::EmbeddedRust
            && __node_3.token_type == LParseToken::LCurlyBrace
            && __node_4.rule == ParseRule::ReservedProductionDefinitionKleene
            && __node_5.token_type == LParseToken::RCurlyBrace
            && __node_6.token_type == LParseToken::Semicolon => {
            let name = __node_0.lexeme;
            let _ = __node_1.lexeme;
            let return_type = __node_2.lexeme;
            let _ = __node_3.lexeme;
            let productions = __rule_factory_function_reserved_production_definition_kleene(
                __node_4,
            );
            let _ = __node_5.lexeme;
            let _ = __node_6.lexeme;
            let _span = node.span();
            LRule {
                name: name,
                return_type: Some(return_type),
                productions,
            }
        }
        (
            ParseRule::LRule,
            [lparse::Node::Leaf(__node_0),
            lparse::Node::Leaf(__node_1),
            lparse::Node::Parent(__node_2),
            lparse::Node::Leaf(__node_3),
            lparse::Node::Leaf(__node_4),
            ],
        ) if true && __node_0.token_type == LParseToken::Ident
            && __node_1.token_type == LParseToken::LCurlyBrace
            && __node_2.rule == ParseRule::ReservedProductionDefinitionKleene
            && __node_3.token_type == LParseToken::RCurlyBrace
            && __node_4.token_type == LParseToken::Semicolon => {
            let name = __node_0.lexeme;
            let _ = __node_1.lexeme;
            let productions = __rule_factory_function_reserved_production_definition_kleene(
                __node_2,
            );
            let _ = __node_3.lexeme;
            let _ = __node_4.lexeme;
            let _span = node.span();
            LRule {
                name: name,
                return_type: None,
                productions,
            }
        }
        _ => panic!("Unreachable"),
    }
}
fn __rule_factory_function_set_goal_rule(node: &lparse::Parent<ParseRule>) -> Ident {
    match (&node.rule, node.children()) {
        (
            ParseRule::SetGoalRule,
            [lparse::Node::Leaf(__node_0),
            lparse::Node::Leaf(__node_1),
            lparse::Node::Leaf(__node_2),
            lparse::Node::Leaf(__node_3),
            ],
        ) if true && __node_0.token_type == LParseToken::GoalRule
            && __node_1.token_type == LParseToken::Equals
            && __node_2.token_type == LParseToken::Ident
            && __node_3.token_type == LParseToken::Semicolon => {
            let _ = __node_0.lexeme;
            let _ = __node_1.lexeme;
            let goalrule = __node_2.lexeme;
            let _ = __node_3.lexeme;
            let _span = node.span();
            goalrule
        }
        _ => panic!("Unreachable"),
    }
}
fn __rule_factory_function_set_token_type(node: &lparse::Parent<ParseRule>) -> Ident {
    match (&node.rule, node.children()) {
        (
            ParseRule::SetTokenType,
            [lparse::Node::Leaf(__node_0),
            lparse::Node::Leaf(__node_1),
            lparse::Node::Leaf(__node_2),
            lparse::Node::Leaf(__node_3),
            ],
        ) if true && __node_0.token_type == LParseToken::TokenType
            && __node_1.token_type == LParseToken::Equals
            && __node_2.token_type == LParseToken::Ident
            && __node_3.token_type == LParseToken::Semicolon => {
            let _ = __node_0.lexeme;
            let _ = __node_1.lexeme;
            let tokentype = __node_2.lexeme;
            let _ = __node_3.lexeme;
            let _span = node.span();
            tokentype
        }
        _ => panic!("Unreachable"),
    }
}
fn __rule_factory_function_preamble(node: &lparse::Parent<ParseRule>) -> Ident {
    match (&node.rule, node.children()) {
        (
            ParseRule::Preamble,
            [lparse::Node::Leaf(__node_0),
            ],
        ) if true && __node_0.token_type == LParseToken::RustImport => {
            let import = __node_0.lexeme;
            let _span = node.span();
            import
        }
        (
            ParseRule::Preamble,
            [lparse::Node::Leaf(__node_0),
            ],
        ) if true && __node_0.token_type == LParseToken::RustDirective => {
            let directive = __node_0.lexeme;
            let _span = node.span();
            directive
        }
        _ => panic!("Unreachable"),
    }
}
fn __rule_factory_function_reserved_l_node_kleene(
    node: &lparse::Parent<ParseRule>,
) -> Vec<LNode> {
    match (&node.rule, node.children()) {
        (ParseRule::ReservedLNodeKleene, []) if true => {
            let _span = node.span();
            Vec::new()
        }
        (
            ParseRule::ReservedLNodeKleene,
            [lparse::Node::Parent(__node_0),
            lparse::Node::Parent(__node_1),
            ],
        ) if true && __node_0.rule == ParseRule::ReservedLNodeKleene
            && __node_1.rule == ParseRule::LNode => {
            let first = __rule_factory_function_reserved_l_node_kleene(__node_0);
            let tail = __rule_factory_function_l_node(__node_1);
            let _span = node.span();
            let mut all = first;
            all.push(tail);
            all
        }
        _ => panic!("Unreachable"),
    }
}
fn __rule_factory_function_reserved_production_definition_kleene(
    node: &lparse::Parent<ParseRule>,
) -> Vec<ProductionDefinition> {
    match (&node.rule, node.children()) {
        (ParseRule::ReservedProductionDefinitionKleene, []) if true => {
            let _span = node.span();
            Vec::new()
        }
        (
            ParseRule::ReservedProductionDefinitionKleene,
            [lparse::Node::Parent(__node_0),
            lparse::Node::Parent(__node_1),
            ],
        ) if true && __node_0.rule == ParseRule::ReservedProductionDefinitionKleene
            && __node_1.rule == ParseRule::ProductionDefinition => {
            let first = __rule_factory_function_reserved_production_definition_kleene(
                __node_0,
            );
            let tail = __rule_factory_function_production_definition(__node_1);
            let _span = node.span();
            let mut all = first;
            all.push(tail);
            all
        }
        _ => panic!("Unreachable"),
    }
}
fn __rule_factory_function_reserved_preamble_kleene(
    node: &lparse::Parent<ParseRule>,
) -> Vec<Ident> {
    match (&node.rule, node.children()) {
        (ParseRule::ReservedPreambleKleene, []) if true => {
            let _span = node.span();
            Vec::new()
        }
        (
            ParseRule::ReservedPreambleKleene,
            [lparse::Node::Parent(__node_0),
            lparse::Node::Parent(__node_1),
            ],
        ) if true && __node_0.rule == ParseRule::ReservedPreambleKleene
            && __node_1.rule == ParseRule::Preamble => {
            let first = __rule_factory_function_reserved_preamble_kleene(__node_0);
            let tail = __rule_factory_function_preamble(__node_1);
            let _span = node.span();
            let mut all = first;
            all.push(tail);
            all
        }
        _ => panic!("Unreachable"),
    }
}
fn __rule_factory_function_reserved_l_rule_kleene(
    node: &lparse::Parent<ParseRule>,
) -> Vec<LRule> {
    match (&node.rule, node.children()) {
        (ParseRule::ReservedLRuleKleene, []) if true => {
            let _span = node.span();
            Vec::new()
        }
        (
            ParseRule::ReservedLRuleKleene,
            [lparse::Node::Parent(__node_0),
            lparse::Node::Parent(__node_1),
            ],
        ) if true && __node_0.rule == ParseRule::ReservedLRuleKleene
            && __node_1.rule == ParseRule::LRule => {
            let first = __rule_factory_function_reserved_l_rule_kleene(__node_0);
            let tail = __rule_factory_function_l_rule(__node_1);
            let _span = node.span();
            let mut all = first;
            all.push(tail);
            all
        }
        _ => panic!("Unreachable"),
    }
}
