window.BENCHMARK_DATA = {
  "lastUpdate": 1780270329328,
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
      },
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
          "id": "29a88ea22ed58c5dd552efe3e94e2388d35c7926",
          "message": "Link benchmark chart from README badge row\n\nThe bench CI job publishes Criterion numbers to\nhttps://jdh8.github.io/ddss/dev/bench/ on every push to main, but\nnothing in the user-facing repo surface pointed there. Adds a fourth\nshields.io badge next to Build/Crates.io/Docs.rs so the chart is\ndiscoverable from the first row a reader scans.\n\nStatic \"benchmarks: published\" label — github-action-benchmark doesn't\nexpose a JSON endpoint for shields.io to read a live number from.",
          "timestamp": "2026-05-25T03:42:30+08:00",
          "tree_id": "b7fd19e20d4bddeb6425437cd4a2f4242e2acf39",
          "url": "https://github.com/jdh8/ddss/commit/29a88ea22ed58c5dd552efe3e94e2388d35c7926"
        },
        "date": 1779652063523,
        "tool": "cargo",
        "benches": [
          {
            "name": "solve_deal_single",
            "value": 73981497,
            "range": "± 183827080",
            "unit": "ns/iter"
          },
          {
            "name": "solve_deals_batch/32",
            "value": 2406416674,
            "range": "± 5079437",
            "unit": "ns/iter"
          },
          {
            "name": "solve_deals_batch/200",
            "value": 16544292079,
            "range": "± 66917688",
            "unit": "ns/iter"
          }
        ]
      },
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
          "id": "d4e0e40edf97dfd16532bd2c9a5fab8407347fc0",
          "message": "ci(readme): split benchmark workflow and update badges",
          "timestamp": "2026-06-01T07:26:31+08:00",
          "tree_id": "79823832faa641d11185b0c337f316e16e798cf5",
          "url": "https://github.com/jdh8/ddss/commit/d4e0e40edf97dfd16532bd2c9a5fab8407347fc0"
        },
        "date": 1780270329012,
        "tool": "cargo",
        "benches": [
          {
            "name": "solve_deal_single",
            "value": 73662647,
            "range": "± 186302228",
            "unit": "ns/iter"
          },
          {
            "name": "solve_deals_batch/32",
            "value": 2501771250,
            "range": "± 8478760",
            "unit": "ns/iter"
          },
          {
            "name": "solve_deals_batch/200",
            "value": 17179536910,
            "range": "± 68484730",
            "unit": "ns/iter"
          }
        ]
      }
    ]
  }
}