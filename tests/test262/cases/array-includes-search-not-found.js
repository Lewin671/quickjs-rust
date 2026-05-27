// Derived from: test/built-ins/Array/prototype/includes/search-not-found-returns-false.js
if ([1, 2, 3].includes(4)) { throw; }
if ([].includes(undefined)) { throw; }
