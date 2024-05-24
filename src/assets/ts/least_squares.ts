interface LinearFit {
    slope: number;
    bias: number;
}

function linearFit(xs: number[], ys: number[]): LinearFit {
    if (xs.length == 0) {
        return { slope: 0, bias: 2.5 };
    }
    // We want to find coefficients [a b] to solve [xs 1's] @ [a b] = [ys].
    // We can do this by solving the linear least squares 2x2 system.

    const lhs = [0, 0, 0, 0];
    const rhs = [0, 0];
    for (let i = 0; i < xs.length; i++) {
        const x = xs[i];
        const y = ys[i];

        // Outer product of [x, 1] with itself.
        lhs[0] += x * x;
        lhs[1] += x;
        lhs[2] += x;
        lhs[3] += 1;

        // Product of [x, 1] with [y].
        rhs[0] += x * y;
        rhs[1] += y;
    }

    // Apply inverse of lhs to rhs.
    const det = lhs[0] * lhs[3] - lhs[1] * lhs[2];
    if (det < 1e-8) {
        // This is a singular matrix, so we just
        // revert to returning the average.
        return { slope: 0, bias: rhs[1] / ys.length };
    }
    const inv = [lhs[3] / det, -lhs[1] / det, -lhs[2] / det, lhs[0] / det];
    const invProduct = [inv[0] * rhs[0] + inv[1] * rhs[1], inv[2] * rhs[0] + inv[3] * rhs[1]];
    return { slope: invProduct[0], bias: invProduct[1] };
}