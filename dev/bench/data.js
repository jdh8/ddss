window.BENCHMARK_DATA = {
  "lastUpdate": 1779650965517,
  "repoUrl": "https://github.com/jdh8/ddss",
  "entries": {
    "Benchmark": [
      {
        "commit": {
          "author": {
            "email": "chen.pang.he@jdh8.org",
            "name": "Chen-Pang He",
            "username": "jdh8"
          },
          "committer": {
            "email": "chen.pang.he@jdh8.org",
            "name": "Chen-Pang He",
            "username": "jdh8"
          },
          "distinct": true,
          "id": "d6f0fe65cba41fa8d260235bcffab20779dda165",
          "message": "Drop N=1000 from solver bench\n\nN=200 already exposes the throughput plateau on a 4-core box (where the\nddss thread pool saturates well before N=1000), and N=1000 was carrying\nno signal beyond that — it just added ~140 s to every full bench run.\n\nmeasurement_time drops from 90 s to 30 s, which still covers 10 samples\nat N=200 (~22 s) with margin and no \"took longer than configured\"\nwarning. The `solve_deals_batch/{32,200}` group remains parameterized\nvia bench_with_input + Throughput::Elements, so per-deal cost across\nsizes stays directly comparable.",
          "timestamp": "2026-05-25T03:24:50+08:00",
          "tree_id": "ace841a4241fa5c3c52d6397db4fbfd66fcc974d",
          "url": "https://github.com/jdh8/ddss/commit/d6f0fe65cba41fa8d260235bcffab20779dda165"
        },
        "date": 1779650965064,
        "tool": "cargo",
        "benches": [
          {
            "name": "solve_deal_single",
            "value": 59044887,
            "range": "± 144862151",
            "unit": "ns/iter"
          },
          {
            "name": "solve_deals_batch/32",
            "value": 1946012342,
            "range": "± 16990264",
            "unit": "ns/iter"
          },
          {
            "name": "solve_deals_batch/200",
            "value": 13068385161,
            "range": "± 40034530",
            "unit": "ns/iter"
          }
        ]
      }
    ]
  }
}