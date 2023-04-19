import csv
from datetime import datetime
import sys

with open(sys.argv[3], "w") as results_f:
    with open(sys.argv[1], "r") as activity_cpu_f:
        activity_cpu = csv.reader(activity_cpu_f, delimiter=" ", skipinitialspace=True)
        next(activity_cpu)

        results_cpu = []
        results_ram = []
        for row in activity_cpu:
            time = row[0]
            cpu = row[1]
            ram = row[2]
            results_cpu.append(f"({time}, {cpu})\n")
            results_ram.append(f"({time}, {ram})\n")

        results_f.write("CPU (%):\n")
        results_f.writelines(results_cpu)
        results_f.write("\n\nRAM (MiB):\n")
        results_f.writelines(results_ram)

    with open(sys.argv[2], "r") as activity_gpu_f:
        activity_gpu = csv.reader(activity_gpu_f, delimiter=",")
        next(activity_gpu)

        results_gpu = []
        results_vram = []
        start_date_time = None
        for row in activity_gpu:
            date_time = datetime.strptime(row[0], "%Y/%m/%d %H:%M:%S.%f")
            if start_date_time is None:
                start_date_time = date_time
            time = str((date_time - start_date_time).total_seconds())
            gpu = row[1]
            vram = row[2]
            results_gpu.append(f"({time}, {gpu})\n")
            results_vram.append(f"({time}, {vram})\n")

        results_f.write("\n\nGPU (%):\n")
        results_f.writelines(results_gpu)
        results_f.write("\n\nVRAM (MiB):\n")
        results_f.writelines(results_vram)
