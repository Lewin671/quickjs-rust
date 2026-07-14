// Rust-native lifecycle fixture v1: medium.
function createLedger(seed) {
  const entries = [];
  let balance = seed;

  function append(kind, amount, metadata) {
    balance += kind === "credit" ? amount : -amount;
    entries.push({ kind, amount, metadata, balance });
    return balance;
  }

  return {
    credit(amount, label) { return append("credit", amount, { label, active: true }); },
    debit(amount, label) { return append("debit", amount, { label, active: false }); },
    summary() {
      let credits = 0;
      let debits = 0;
      for (let index = 0; index < entries.length; index += 1) {
        const entry = entries[index];
        if (entry.kind === "credit") {
          credits += entry.amount;
        } else {
          debits += entry.amount;
        }
      }
      return { balance, credits, debits, count: entries.length };
    }
  };
}

function buildSeries(length) {
  const series = [];
  for (let index = 0; index < length; index += 1) {
    const base = index * 3 + 1;
    series.push({ id: index, values: [base, base + 1, base + 2] });
  }
  return series;
}

const ledger = createLedger(100);
const series = buildSeries(24);
let checksum = 0;

for (let row = 0; row < series.length; row += 1) {
  const item = series[row];
  for (let column = 0; column < item.values.length; column += 1) {
    const value = item.values[column];
    if ((row + column) % 3 === 0) {
      checksum += ledger.credit(value, "batch-" + row);
    } else {
      checksum += ledger.debit(value % 5, "fee-" + column);
    }
  }
}

const summary = ledger.summary();
checksum + summary.balance + summary.credits - summary.debits;
