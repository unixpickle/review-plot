function linearFit(xs, ys) {
    if (xs.length == 0) {
        return { slope: 0, bias: 2.5 };
    }
    const lhs = [0, 0, 0, 0];
    const rhs = [0, 0];
    for (let i = 0; i < xs.length; i++) {
        const x = xs[i];
        const y = ys[i];
        lhs[0] += x * x;
        lhs[1] += x;
        lhs[2] += x;
        lhs[3] += 1;
        rhs[0] += x * y;
        rhs[1] += y;
    }
    const det = lhs[0] * lhs[3] - lhs[1] * lhs[2];
    if (det < 1e-8) {
        return { slope: 0, bias: rhs[1] / ys.length };
    }
    const inv = [lhs[3] / det, -lhs[1] / det, -lhs[2] / det, lhs[0] / det];
    const invProduct = [inv[0] * rhs[0] + inv[1] * rhs[1], inv[2] * rhs[0] + inv[3] * rhs[1]];
    return { slope: invProduct[0], bias: invProduct[1] };
}
//# sourceMappingURL=least_squares.js.map