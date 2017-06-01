use corollary_support::*;

use Language::C::Data::Ident;
use Language::C::Data::Node;
use Language::C::Analysis::DefTable;
use Language::C::Analysis::SemRep;
use Language::C::Analysis::TypeUtils;

pub fn builtins() -> DefTable {
    foldr(doIdent, (foldr(doTypeDef, emptyDefTable, typedefs)), idents)
}


