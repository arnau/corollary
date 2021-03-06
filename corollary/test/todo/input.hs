module Test.Code ()
where

bitWidth :: Int -> IntWidth -> Int
bitWidth wordWidth WordWidth = wordWidth
bitWidth _ (BitWidth w) = w

blockToStatements :: Rust.Block -> [Rust.Stmt]
blockToStatements (Rust.Block stmts mexpr) = case mexpr of
    Just expr -> stmts ++ exprToStatements expr
    Nothing -> stmts

addExternIdent
    :: Ident
    -> EnvMonad s IntermediateType
    -> (String -> (Rust.Mutable, CType) -> Rust.ExternItem)
    -> EnvMonad s ()
addExternIdent ident deferred mkItem = do
    action <- runOnce $ do
        itype <- deferred
        rewrites <- lift $ asks itemRewrites
        path <- case Map.lookup (Symbol, identToString ident) rewrites of
            Just renamed -> return ("" : renamed)
            Nothing -> return [name]
    addSymbolIdentAction ident action

addSymbolIdent :: Ident -> (Rust.Mutable, CType) -> EnvMonad s String
addSymbolIdent ident (mut, ty) = do
    let name = applyRenames ident
    addSymbolIdentAction ident $ return Result
        { resultType = ty
        , resultMutable = mut
        , result = Rust.Path (Rust.PathSegments [name])
        }
    return name

-- Main lexing routines
data AlexReturn a
  = AlexEOF
  | AlexError !AlexInput
  | AlexSkip !AlexInput
             !Int
  | AlexToken !AlexInput
              !Int
              a

sumEuler :: Int -> Int
sumEuler = sum . (map euler) . mkList

interpretDeclarations :: MakeBinding s b -> CDecl -> EnvMonad s [b]
interpretDeclarations (fromItem, makeBinding) declaration@(CDecl specs decls _) = do
    (storagespecs, baseTy) <- baseTypeOf specs
    mbinds <- forM decls $ \ declarator -> do
        (decl, minit) <- case declarator of
            (Just decl, minit, Nothing) -> return (decl, minit)
            (Nothing, _, _) -> badSource declaration "absent declarator"
            (_, _, Just _) -> badSource declaration "bitfield declarator"

        (ident, derived) <- case decl of
            CDeclr (Just ident) derived _ _ _ -> return (ident, derived)
            _ -> badSource decl "abstract declarator"

        deferred <- derivedDeferredTypeOf baseTy decl []
        case (storagespecs, derived) of
            (Just (CTypedef _), _) -> do
                when (isJust minit) (badSource decl "initializer on typedef")
                addTypedefIdent ident deferred
                return Nothing
            (Just (CStatic _), CFunDeclr{} : _) -> do
                addSymbolIdentAction ident $ do
                    itype <- deferred
                    useForwardRef ident
                    return (typeToResult itype (Rust.Path (Rust.PathSegments [applyRenames ident])))
                return Nothing
            (_, CFunDeclr{} : _) -> do
                addExternIdent ident deferred $ \ name (_mut, ty) -> case ty of
                    IsFunc retTy args variadic ->
                        -- let { f = list comprehension } in
                        let formals =
                                [ (Rust.VarName argName, toRustType argTy)
                                | (idx, (mname, argTy)) <- zip [1 :: Int ..] args
                                , let argName = maybe ("arg" ++ show idx) (applyRenames . snd) mname
                                ]
                        in Rust.ExternFn name formals variadic (toRustRetType retTy)
                    _ -> error (show ident ++ " is both a function and not a function?")
                return Nothing
    -- [...]
    return (catMaybes mbinds)
interpretDeclarations _ node@(CStaticAssert {}) = unimplemented node

{-RUST
fn main() {
    println!("success.");
}
/RUST-}
