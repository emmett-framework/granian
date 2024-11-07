{{
    def get_max_concurrency_run(data, k1="requests", k2="rps"):
        concurrency_values = {data[ckey][k1][k2]: ckey for ckey in data.keys()}
        maxc = concurrency_values[max(concurrency_values.keys())]
        return maxc, data[maxc]

    def fmt_ms(v):
        return f"{round(v, 3)}ms" if v else "N/A"
}}
