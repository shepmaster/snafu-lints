#![feature(rustc_private)]
#![deny(rust_2018_idioms)]

extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

use clippy_utils::diagnostics::span_lint_and_help;
use rustc_hir::def::Res;
use rustc_hir::{
    def_id::DefId,
    intravisit::{walk_expr, Visitor},
    Expr, ExprField, ExprKind,
};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::hir::nested_filter::OnlyBodies;
use rustc_span::symbol::Symbol;
use rustc_span::Span;

dylint_linting::impl_late_lint! {
    #[doc = include_str!("../README.md")]
    pub SNAFU_CONTEXT_FUN_CALL,
    Warn,
    "description goes here",
    SnafuContextFunCall::default()
}

const SNAFU_FUTURES_TRY_FUTURE_TRYFUTUREEXT_CONTEXT: [&str; 5] =
    ["snafu", "futures", "try_future", "TryFutureExt", "context"];
const SNAFU_FUTURES_TRY_STREAM_TRYSTREAMEXT_CONTEXT: [&str; 5] =
    ["snafu", "futures", "try_stream", "TryStreamExt", "context"];
const SNAFU_OPTIONEXT_CONTEXT: [&str; 3] = ["snafu", "OptionExt", "context"];
const SNAFU_RESULTEXT_CONTEXT: [&str; 3] = ["snafu", "ResultExt", "context"];
const CONTEXT_LIKE_METHODS: &[&[&str]] = &[
    &SNAFU_FUTURES_TRY_FUTURE_TRYFUTUREEXT_CONTEXT,
    &SNAFU_FUTURES_TRY_STREAM_TRYSTREAMEXT_CONTEXT,
    &SNAFU_OPTIONEXT_CONTEXT,
    &SNAFU_RESULTEXT_CONTEXT,
];

const ALLOC_STRING_STRING_AS_STR: [&str; 4] = ["alloc", "string", "String", "as_str"];
const CORE_CONVERT_ASREF_AS_REF: [&str; 4] = ["core", "convert", "AsRef", "as_ref"];
const ALLOWED_METHODS: &[&[&str]] = &[&ALLOC_STRING_STRING_AS_STR, &CORE_CONVERT_ASREF_AS_REF];

#[derive(Debug, Default, serde::Deserialize)]
struct Config {
    allow: Vec<String>,
}

struct SnafuContextFunCall {
    context_like: Vec<Vec<Symbol>>,
    allowed_calls: Vec<Vec<Symbol>>,
}

impl Default for SnafuContextFunCall {
    fn default() -> Self {
        let config: Config = dylint_linting::config_or_default(env!("CARGO_PKG_NAME"));

        let context_like = intern_paths(CONTEXT_LIKE_METHODS);
        let mut allowed_calls = intern_paths(ALLOWED_METHODS);

        let allowed_by_user = intern_user_paths(&config.allow);
        allowed_calls.extend(allowed_by_user);

        Self {
            context_like,
            allowed_calls,
        }
    }
}

fn intern_paths(paths: &[&[&str]]) -> Vec<Vec<Symbol>> {
    paths
        .iter()
        .map(|&path| path.iter().map(|c| Symbol::intern(c)).collect())
        .collect()
}

fn intern_user_paths(paths: &[impl AsRef<str>]) -> Vec<Vec<Symbol>> {
    paths
        .iter()
        .map(|path| {
            path.as_ref()
                .split("::")
                .map(|c| Symbol::intern(c.trim()))
                .collect()
        })
        .collect()
}

impl<'tcx> LateLintPass<'tcx> for SnafuContextFunCall {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        let Some(fields) = call_to_context_like(cx, expr, &self.context_like) else { return };

        for field in fields {
            for call in forbidden_calls_within(cx, field.expr, &self.allowed_calls) {
                let name = call.name();

                span_lint_and_help(
                    cx,
                    SNAFU_CONTEXT_FUN_CALL,
                    call.span(),
                    &format!(
                        "Context selector field expressions should \
                        avoid {name} calls when used with `context` as \
                        the {name} calls will be invoked even when no \
                        error is created",
                    ),
                    None,
                    &format!(
                        "Replace `context` with `with_context` or \
                        replace the {name} call with an existing \
                        value. Context selectors will automatically \
                        call `Into::into` on the field expression as \
                        needed, so an explicit {name} call can often \
                        be avoided.",
                    ),
                );
            }
        }
    }
}

/// Finds function or method calls to `ResultExt::context` (and
/// similar functions/traits) and returns the fields of the context
/// selector.
fn call_to_context_like<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'tcx>,
    context_like: &[Vec<Symbol>],
) -> Option<&'tcx [ExprField<'tcx>]> {
    let args = match expr.kind {
        ExprKind::Call(_path, args) => args,
        ExprKind::MethodCall(_path, _receiver, args, _span) => args,
        _ => return None,
    };
    let [arg] = args else { return None };
    let ExprKind::Struct(_qpath, fields, _base) = arg.kind else { return None };

    if !is_any_call_path(cx, expr, context_like) {
        return None;
    }

    Some(fields)
}

/// Explores an expression for any function or method calls that are
/// not in the allow list.
fn forbidden_calls_within<'cx, 'tcx>(
    cx: &'cx LateContext<'tcx>,
    expr: &'tcx Expr<'tcx>,
    allowed: &'cx [Vec<Symbol>],
) -> impl Iterator<Item = CallKind<'tcx>> {
    let mut v = ForbiddenCallsWithinVisitor {
        cx,
        found: Default::default(),
        allowed,
    };
    v.visit_expr(expr);
    v.found.into_iter()
}

struct ForbiddenCallsWithinVisitor<'cx, 'tcx> {
    cx: &'cx LateContext<'tcx>,
    found: Vec<CallKind<'tcx>>,
    allowed: &'cx [Vec<Symbol>],
}

impl<'cx, 'tcx> Visitor<'tcx> for ForbiddenCallsWithinVisitor<'cx, 'tcx> {
    type NestedFilter = OnlyBodies;

    fn nested_visit_map(&mut self) -> Self::Map {
        self.cx.tcx.hir()
    }

    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) {
        match &expr.kind {
            ExprKind::Call(fn_expr, _args) => {
                if !self.is_allowed_call(fn_expr) {
                    self.found.push(CallKind::Function(expr));
                }
            }

            ExprKind::MethodCall(_path, _receiver, _args, _span) => {
                if !self.is_allowed_call(expr) {
                    self.found.push(CallKind::Method(expr));
                }
            }

            _ => walk_expr(self, expr),
        }
    }
}

impl<'cx, 'tcx> ForbiddenCallsWithinVisitor<'cx, 'tcx> {
    fn is_allowed_call(&self, call_expr: &'tcx Expr<'tcx>) -> bool {
        is_any_call_path(self.cx, call_expr, self.allowed)
    }
}

fn is_any_call_path<'tcx>(
    cx: &LateContext<'tcx>,
    call_expr: &'tcx Expr<'tcx>,
    paths: &[Vec<Symbol>],
) -> bool {
    let Some(call_def_id) = call_def_id(cx, call_expr) else { return false };

    paths
        .iter()
        .any(|path| match_def_path_symbol(cx, call_def_id, path))
}

// Same as Clippy's match_def_path but takes a `Symbol` so we can intern early.
fn match_def_path_symbol<'tcx>(cx: &LateContext<'tcx>, did: DefId, syms: &[Symbol]) -> bool {
    let path = cx.get_def_path(did);
    syms.iter().eq(path.iter())
}

fn call_def_id<'tcx>(cx: &LateContext<'tcx>, call_expr: &'tcx Expr<'tcx>) -> Option<DefId> {
    let typeck_results = cx.typeck_results();

    typeck_results.type_dependent_def_id(call_expr.hir_id).or_else(|| {
        // `TypeckResults::type_dependent_def_id` doesn't appear to
        // handle `<_ as Trait>::method`?
        let ExprKind::Path(path) = &call_expr.kind else { return None };
        let Res::Def(_, def_id) = typeck_results.qpath_res(path, call_expr.hir_id) else { return None };
        Some(def_id)
    })
}

enum CallKind<'tcx> {
    Function(&'tcx Expr<'tcx>),
    Method(&'tcx Expr<'tcx>),
}

impl<'tcx> CallKind<'tcx> {
    fn name(&self) -> &'static str {
        match self {
            CallKind::Function(..) => "function",
            CallKind::Method(..) => "method",
        }
    }

    fn expr(&self) -> &'tcx Expr<'tcx> {
        match self {
            CallKind::Function(e) => e,
            CallKind::Method(e) => e,
        }
    }

    fn span(&self) -> Span {
        self.expr().span
    }
}

#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
