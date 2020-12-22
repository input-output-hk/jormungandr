#!/usr/bin/python3
# -*- coding: utf-8 -*-

import subprocess
import json
import tempfile


def jcli_generate_share(encrypted_tally_path, decryption_key_path):
    cli_args = [
        "jcli", "votes", "tally", "decryption-share",
        "--encrypted-tally", encrypted_tally_path,
        "--key", decryption_key_path
    ]
    try:
        result = subprocess.check_output(cli_args)
        return json.loads(result)
    except subprocess.CalledProcessError as e:
        print(f"Error executing process, exit code {e.returncode}:\n{e.output}")


def jcli_decrypt_tally(encrypted_tally_path, shares_path, threshold, max_votes, table_size, output_format="json"):
    cli_args = [
        "jcli", "votes", "tally", "decrypt",
        "--encrypted-tally", encrypted_tally_path,
        "--shares", shares_path,
        "--threshold", threshold,
        "--max-votes", max_votes,
        "--table-size", table_size,
        "--output-format", output_format
    ]
    try:
        result = subprocess.check_output(cli_args)
        if output_format.lower() == "json":
            return json.loads(result)
        return result
    except subprocess.CalledProcessError as e:
        print(f"Error executing process, exit code {e.returncode}:\n{e.output}")


def jcli_request_active_voteplans(output_format="json"):
    cli_args = [
        "jcli", "rest", "v0", "vote", "plans", "get"
    ]
    try:
        result = subprocess.check_output(cli_args)
        if output_format.lower() == "json":
            return json.loads(result)
        return result
    except subprocess.CalledProcessError as e:
        print(f"Error executing process, exit code {e.returncode}:\n{e.output}")


def load_json_file(file_path):
    try:
        with open(file_path) as f:
            return json.load(f)
    except FileNotFoundError:
        print(f"File {file_path} coul not be found.")
    except json.JSONDecodeError as e:
        print(f"Malformed json:\n{e}")


def generate_committee_member_shares(decryption_key_path, input_file=None, output_file="./proposals.shares"):
    active_vote_plans = (
        jcli_request_active_voteplans() if input_file is None else load_json_file(input_file)
    )


    # Active voteplans dict would look like:
    # {
    #   id: Hash,
    #   payload: PauloadType,
    #   vote_start: BlockDate,
    #   vote_end: BlcokDate,
    #   committee_end: BlockDate,
    #   committee_member_keys: [MemberPublicKey]
    #   proposals: [
    #       {
    #           index: int,
    #           proposal_id: Hash,
    #           options: [u8],
    #           tally: Tally(Public or Private),
    #           votes_cast: int,
    #       },
    #   ]
    # }
    proposals = active_vote_plans["proposals"]
    for proposal in proposals:
        try:
            encrypted_tally = proposal["tally"]["private"]["encrypted"]["encrypted_tally"]
        except KeyError:
            raise Exception(f"Tally data wasn't expected:\n{proposal}")
        f, tmp_tally_path = tempfile.mkstemp()
        with open(tmp_tally_path, "w") as f:
            f.write(encrypted_tally)
        # result is of format:
        # {
        #   state: base64,
        #   share: base64
        # }
        result = jcli_generate_share(tmp_tally_path, decryption_key_path)
        proposal["shares"] = result["share"]

    with open(output_file, "w") as f:
        json.dump(f, proposals, indent=4)
    print(f"Shares file processed properly at: {output_file}")


def merge_generated_shares(*share_files_paths, output_file="aggregated_shares.shares"):
    from functools import reduce

    def load_data(path):
        with open(path) as f:
            return json.load(f)

    def merge_two_shares_data(data1, data2):
        data1["shares"].append(data2.pop())
        return data1

    shares_data = (load_data(p) for p in share_files_paths)
    full_data = reduce(merge_two_shares_data, shares_data)

    with open(output_file, "w") as f:
        json.dump(f, full_data)

    print(f"Data succesfully aggregated at: {output_file}")


def tally_with_shares(aggregated_data_shares_file, output_file="decrypted_tally"):

    def write_shares(f, shares):
        for s in shares:
            f.writeline(s)

    def write_tally(f, tally):
        f.write(tally)

    with open(aggregated_data_shares_file) as f:
        try:
            aggregated_data = json.load(f)
        except ValueError:
            raise Exception(f"Error loading data from file: {aggregated_data_shares_file}")

    for data in aggregated_data:
        try:
            encrypted_tally = data["tally"]["private"]["encrypted"]["encrypted_tally"]
            shares = data["shares"]
        except KeyError:
            raise Exception(f"Tally data wasn't expected:\n{data}")

        _, tmp_tally_file = tempfile.mkstemp()
        _, tmp_shares_file = tempfile.mkstemp()

        with open(tmp_tally_file, "w") as tally_f, open(tmp_shares_file, "w") as shares_f:
            write_tally(tally_f, encrypted_tally)
            write_shares(shares_f, shares)

        threshold = len(shares)
        max_votes = data["votes_cast"]
        options = data["options"]["end"]
        table_size = max_votes // options

        result = jcli_decrypt_tally(
            tmp_tally_file, tmp_shares_file, threshold, max_votes, table_size
        )

        data["tally"]["tally_result"] = result

    with open(output_file, "w") as f:
        json.dump(f, aggregated_data)

    print("Tally successfully decrypted")


if __name__ == "__main__":
    import click


    @click.group()
    def cli():
        pass


    @cli.command()
    @click.option("--key", type=str, help="Path to committee member key file")
    @click.option("--output", type=str, help="Output path for the share file", default="./proposals.shares")
    @click.option(
        "--input-file",
        type=str,
        help="Path to the active voteplans json file. If not provided it will be requested directly to the node",
        default=None
    )
    def generate_share(key, input_file, output):
        generate_committee_member_shares(key, input_file, output)


    @cli.command()
    @click.option("--shares", "-s", type=str, multiple=True, help="Path to a generated proposal shares file")
    @click.option("--output", type=str, help="Output path for the merged shares file", default="./merged_proposals.shares")
    def merge_shares(shares, output):
        merge_generated_shares(*shares, output_file=output)


    @cli.command()
    @click.option("--shares", type=str, help="Path to the merged shares file")
    @click.option("--output", type=str, help="Output path for the merged shares file", default="./results.json")
    def tally(shares, output):
        tally_with_shares(shares, output)


    cli()
