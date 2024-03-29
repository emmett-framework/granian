wrk.method = "POST"
wrk.body   = "Test"
wrk.headers["Content-Type"] = "text/plain; charset=utf-8"

done = function(summary, latency, requests)

    out = {
      summary.duration,
      summary.requests,
      summary.requests/(summary.duration/1000000),
      summary.bytes,
      summary.errors.connect,
      summary.errors.read,
      summary.errors.write,
      summary.errors.status,
      summary.errors.timeout,
      latency.min,
      latency.max,
      latency.mean,
      latency.stdev,
      latency:percentile(50),
      latency:percentile(75),
      latency:percentile(90),
      latency:percentile(99),
      latency:percentile(99.999)
    }

    for key, value in pairs(out) do
      if key > 1 then
        io.stderr:write(",")
      end
      io.stderr:write(string.format("%d", value))
    end

  end
