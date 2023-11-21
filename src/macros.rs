/// Helper to efficiently repeat type parameters.
macro_rules! repeat {
    ($macro:path) => {
        $macro!(A);
        $macro!(A, B);
        $macro!(A, B, C);
        $macro!(A, B, C, D);
        $macro!(A, B, C, D, E);
        $macro!(A, B, C, D, E, F);
        $macro!(A, B, C, D, E, F, G);
        $macro!(A, B, C, D, E, F, G, H);
        $macro!(A, B, C, D, E, F, G, H, I);
        $macro!(A, B, C, D, E, F, G, H, I, J);
        $macro!(A, B, C, D, E, F, G, H, I, J, K);
        $macro!(A, B, C, D, E, F, G, H, I, J, K, L);
        $macro!(A, B, C, D, E, F, G, H, I, J, K, L, M);
        $macro!(A, B, C, D, E, F, G, H, I, J, K, L, M, N);
        $macro!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O);
        $macro!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P);
    };
}
