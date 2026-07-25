async function main() {
    for (let i = 1; i < 101; i++) {
        const result = (function() {
            const __m = [i % 3, i % 5];
            if (__m[0] === 0 && __m[1] === 0) return `FizzBuzz`;
            if (__m[0] === 0 && true) return `Fizz`;
            if (true && __m[1] === 0) return `Buzz`;
            return i;
            throw new Error('Non-exhaustive match');
        })();
        console.log(result);
    }
}


if (typeof main === 'function') { main(); }