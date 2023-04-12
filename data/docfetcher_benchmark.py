import csv
import sys
import time

from py4j.java_gateway import JavaGateway, GatewayParameters
from py4j.java_gateway import java_import

MAX_RANK = 100

with open(sys.argv[1], "r") as queries_f:
    queries_tsv = csv.reader(queries_f, delimiter="\t")

    with open(sys.argv[2], "w") as results_f:
        results_csv = csv.writer(results_f, delimiter=" ")

        gateway = JavaGateway(gateway_parameters=GatewayParameters(port=28834))
        java_import(gateway.jvm, "net.sourceforge.docfetcher.gui.Application")
        application = gateway.jvm.net.sourceforge.docfetcher.gui.Application  # type: ignore

        indexRegistry = application.getIndexRegistry()  # type: ignore
        searcher = indexRegistry.getSearcher()  # type: ignore

        for query_row in queries_tsv:
            query_id = query_row[0]
            query_text = query_row[1]

            time_start = time.perf_counter()
            results = searcher.search(query_text.replace("/", " "))[:MAX_RANK]
            duration_s = time.perf_counter() - time_start

            for i, doc in enumerate(results):
                results_csv.writerow(
                    (
                        query_id,
                        0,
                        doc.getFilename()[:-4],
                        i,
                        MAX_RANK - i,
                        0,
                        duration_s,
                    )
                )

            print(f"Processed query {query_id}")
