{{
    def get_max_concurrency_run(data):
        concurrency_values = {data[ckey]["requests"]["rps"]: ckey for ckey in data.keys()}
        maxc = concurrency_values[max(concurrency_values.keys())]
        return maxc, data[maxc]

    def fmt_ms(v):
        return f"{round(v, 3)}ms" if v else "N/A"
}}
