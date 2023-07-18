import os, shutil, json

with open('latest_test.json', 'r') as file:
    json_data = [json.loads(line) for line in file]

test_binary_path = json_data[-2].get("filenames", [None])[0]
if test_binary_path:
    target_directory = 'target/test'

    os.makedirs(target_directory, exist_ok=True)
    shutil.copy(test_binary_path, os.path.join(target_directory, 'latest_tests'))
else:
    print("An error has occured. Wrong json format!")