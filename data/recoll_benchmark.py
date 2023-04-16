import csv
import sys
import time

from recoll import recoll

MAX_RANK = 100

with open(sys.argv[1], "r") as queries_f:
    queries_tsv = csv.reader(queries_f, delimiter="\t")

    with open(sys.argv[2], "w") as results_f:
        results_csv = csv.writer(results_f, delimiter=" ")

        db = recoll.connect()  # type: ignore

        for query_row in queries_tsv:
            query_id = query_row[0]
            query_text = query_row[1]

            time_start = time.perf_counter()

            query = db.query()
            query.execute(
                " OR ".join(
                    query_text.replace(".", "")
                    .replace(",", "")
                    .replace(":", "")
                    .replace("?", "")
                    .replace("(", "")
                    .replace(")", "")
                    .replace(" - ", " ")
                    .replace("---", " ")
                    .replace("/", " ")
                    .split()
                ),
                stemming=1,
                # stemlang="russian",
            )

            results = query.fetchmany(MAX_RANK)
            results_dedup = []
            results_ids = set()
            for doc in results:
                if doc.filename in results_ids:
                    results.append(query.fetchone())
                else:
                    results_ids.add(doc.filename)
                    results_dedup.append(doc)

            query.close()

            duration_s = time.perf_counter() - time_start

            for i, doc in enumerate(results_dedup):
                results_csv.writerow(
                    (query_id, 0, doc.filename[:-4], i, MAX_RANK - i, 0, duration_s)
                )

            print(f"Processed query {query_id}")
