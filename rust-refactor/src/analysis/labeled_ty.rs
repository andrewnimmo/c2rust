use std::fmt;
use std::marker::PhantomData;
use arena::DroplessArena;
use rustc::ty::{Ty, TyCtxt, FnSig, TypeVariants};
use rustc::ty::subst::Substs;

use type_map;


#[derive(Clone, PartialEq, Eq)]
pub struct LabeledTyS<'tcx, L: 'tcx> {
    pub ty: Ty<'tcx>,
    pub args: &'tcx [LabeledTy<'tcx, L>],
    pub label: L,
}

pub type LabeledTy<'tcx, L> = &'tcx LabeledTyS<'tcx, L>;

impl<'tcx, L: fmt::Debug> fmt::Debug for LabeledTyS<'tcx, L> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}#{:?}{:?}", self.label, self.ty, self.args)
    }
}

impl<'tcx, L> LabeledTyS<'tcx, L> {
    pub fn for_each_label<F: FnMut(&'tcx L)>(&'tcx self, callback: &mut F) {
        callback(&self.label);
        for &arg in self.args {
            arg.for_each_label(callback);
        }
    }
}


pub struct LabeledTyCtxt<'tcx, L: 'tcx> {
    arena: &'tcx DroplessArena,
    _marker: PhantomData<L>,
}

impl<'tcx, L: Clone> LabeledTyCtxt<'tcx, L> {
    pub fn new(arena: &'tcx DroplessArena) -> LabeledTyCtxt<'tcx, L> {
        LabeledTyCtxt {
            arena: arena,
            _marker: PhantomData,
        }
    }

    pub fn mk_slice(&self, ltys: &[LabeledTy<'tcx, L>]) -> &'tcx [LabeledTy<'tcx, L>] {
        if ltys.len() == 0 {
            return &[];
        }
        self.arena.alloc_slice(ltys)
    }

    pub fn mk(&self, ty: Ty<'tcx>, args: &'tcx [LabeledTy<'tcx, L>], label: L) -> LabeledTy<'tcx, L> {
        self.arena.alloc(LabeledTyS {
            ty: ty,
            args: args,
            label: label,
        })
    }


    pub fn label<F: FnMut(Ty<'tcx>) -> L>(&self, ty: Ty<'tcx>, f: &mut F) -> LabeledTy<'tcx, L> {
        use rustc::ty::TypeVariants::*;
        let label = f(ty);
        match ty.sty {
            // Types with no arguments
            TyBool |
            TyChar |
            TyInt(_) |
            TyUint(_) |
            TyFloat(_) |
            TyStr |
            TyNever => self.mk(ty, &[], label),

            // Types with arguments
            TyAdt(_, substs) => {
                let args = substs.types().map(|t| self.label(t, f)).collect::<Vec<_>>();
                self.mk(ty, self.mk_slice(&args), label)
            },
            TyArray(elem, _) => {
                let args = [self.label(elem, f)];
                self.mk(ty, self.mk_slice(&args), label)
            },
            TySlice(elem) => {
                let args = [self.label(elem, f)];
                self.mk(ty, self.mk_slice(&args), label)
            },
            TyRawPtr(mty) => {
                let args = [self.label(mty.ty, f)];
                self.mk(ty, self.mk_slice(&args), label)
            },
            TyRef(_, mty) => {
                let args = [self.label(mty.ty, f)];
                self.mk(ty, self.mk_slice(&args), label)
            },
            TyFnDef(_, substs) => {
                let args = substs.types().map(|ty| self.label(ty, f)).collect::<Vec<_>>();
                self.mk(ty, self.mk_slice(&args), label)
            },
            TyFnPtr(ref sig) => {
                let args = sig.0.inputs_and_output.iter()
                    .map(|ty| self.label(ty, f)).collect::<Vec<_>>();
                self.mk(ty, self.mk_slice(&args), label)
            },
            TyTuple(ref elems, _) => {
                let args = elems.iter().map(|ty| self.label(ty, f)).collect::<Vec<_>>();
                self.mk(ty, self.mk_slice(&args), label)
            },

            // Types that aren't actually supported by this code yet
            TyDynamic(..) |
            TyClosure(..) |
            TyProjection(..) |
            TyAnon(..) |
            TyParam(..) |
            TyInfer(..) |
            TyError => self.mk(ty, &[], label),
        }
    }

    pub fn label_slice<F>(&self,
                          tys: &[Ty<'tcx>],
                          f: &mut F) -> &'tcx [LabeledTy<'tcx, L>]
            where F: FnMut(Ty<'tcx>) -> L {
        self.mk_slice(&tys.iter().map(|ty| self.label(ty, f)).collect::<Vec<_>>())
    }


    pub fn subst(&self,
                 lty: LabeledTy<'tcx, L>,
                 substs: &[LabeledTy<'tcx, L>]) -> LabeledTy<'tcx, L> {
        match lty.ty.sty {
            TypeVariants::TyParam(ref tp) => {
                substs[tp.idx as usize]
            },
            _ => self.mk(lty.ty, self.subst_slice(lty.args, substs), lty.label.clone()),
        }
    }

    pub fn subst_slice(&self,
                       ltys: &[LabeledTy<'tcx, L>],
                       substs: &[LabeledTy<'tcx, L>]) -> &'tcx [LabeledTy<'tcx, L>] {
        self.mk_slice(&ltys.iter().map(|lty| self.subst(lty, substs)).collect::<Vec<_>>())
    }


    pub fn relabel<L2, F>(&self, lty: LabeledTy<'tcx, L2>, func: &mut F) -> LabeledTy<'tcx, L>
            where F: FnMut(&L2) -> L {
        let args = self.relabel_slice(lty.args, func);
        self.mk(lty.ty, args, func(&lty.label))
    }

    pub fn relabel_slice<L2, F>(&self,
                                ltys: &'tcx [LabeledTy<'tcx, L2>],
                                func: &mut F) -> &'tcx [LabeledTy<'tcx, L>]
            where F: FnMut(&L2) -> L {
        let ltys = ltys.iter().cloned().map(|lty| self.relabel(lty, func)).collect::<Vec<_>>();
        self.mk_slice(&ltys)
    }
}


impl<'tcx, L: fmt::Debug> type_map::Type for LabeledTy<'tcx, L> {
    fn sty(&self) -> &TypeVariants {
        &self.ty.sty
    }

    fn num_args(&self) -> usize {
        self.args.len()
    }

    fn arg(&self, idx: usize) -> Self {
        self.args[idx]
    }
}