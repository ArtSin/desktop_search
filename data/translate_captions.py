import json
import sys

import tqdm
from transformers import AutoModelForSeq2SeqLM, AutoTokenizer

model_name = "facebook/nllb-200-distilled-1.3B"
device = "cuda"
tokenizer = AutoTokenizer.from_pretrained(model_name)
model = AutoModelForSeq2SeqLM.from_pretrained(model_name).half().to(device)

with open(sys.argv[1], "r") as file:
    captions_json = json.load(file)

translated = []
with tqdm.tqdm(captions_json["annotations"]) as t:
    for x in t:
        input = x["caption"]
        input_ids = tokenizer(input, return_tensors="pt").to(device)
        output = model.generate(
            **input_ids,
            forced_bos_token_id=tokenizer.lang_code_to_id["rus_Cyrl"],
            max_length=200
        )
        decoded = tokenizer.batch_decode(output, skip_special_tokens=True)[0]
        translated.append(
            {"image_id": x["image_id"], "id": x["id"], "caption": decoded}
        )

with open(sys.argv[2], "w") as file:
    json.dump({"annotations": translated}, file, ensure_ascii=False)
