#!/usr/bin/env python3
"""Canonical Trading OpenAPI normalization shared by coverage tools."""

from __future__ import annotations

import copy
import hashlib
import json
import pathlib
import urllib.request
from typing import Any, Iterator


HTTP_METHODS = ("delete", "get", "head", "options", "patch", "post", "put", "trace")
CANONICAL_OPENAPI_URL = "https://docs.alpaca.markets/us/openapi/trading-api.json"


def load_openapi(source: str) -> tuple[dict[str, Any], bytes]:
    if source.startswith("file://"):
        payload = pathlib.Path(source.removeprefix("file://")).read_bytes()
    elif pathlib.Path(source).is_file():
        payload = pathlib.Path(source).read_bytes()
    else:
        request = urllib.request.Request(
            source, headers={"User-Agent": "alpaca-rust-coverage-audit"}
        )
        with urllib.request.urlopen(request, timeout=30) as response:
            payload = response.read()
    return json.loads(payload), payload


def sha256(payload: bytes) -> str:
    return hashlib.sha256(payload).hexdigest()


def iter_operations(
    document: dict[str, Any],
) -> Iterator[tuple[str, str, dict[str, Any], dict[str, Any]]]:
    for path in sorted(document.get("paths", {})):
        path_item = document["paths"][path]
        for method in HTTP_METHODS:
            operation = path_item.get(method)
            if operation is not None:
                yield method.upper(), path, path_item, operation


def operation_count(document: dict[str, Any]) -> int:
    return sum(1 for _ in iter_operations(document))


def _pointer_target(document: dict[str, Any], reference: str) -> dict[str, Any]:
    if not reference.startswith("#/"):
        raise ValueError(f"unsupported external OpenAPI reference: {reference}")
    current: Any = document
    for raw_segment in reference[2:].split("/"):
        segment = raw_segment.replace("~1", "/").replace("~0", "~")
        current = current[segment]
    if not isinstance(current, dict):
        raise ValueError(
            f"OpenAPI reference does not resolve to an object: {reference}"
        )
    return current


def dereference(document: dict[str, Any], node: Any) -> Any:
    current = node
    seen: set[str] = set()
    while isinstance(current, dict) and "$ref" in current:
        reference = current["$ref"]
        if reference in seen:
            return current
        seen.add(reference)
        resolved = copy.deepcopy(_pointer_target(document, reference))
        overlay = {
            key: copy.deepcopy(value) for key, value in current.items() if key != "$ref"
        }
        current = _merge_schema(resolved, overlay)
    return current


def _merge_schema(left: dict[str, Any], right: dict[str, Any]) -> dict[str, Any]:
    merged = copy.deepcopy(left)
    for key, value in right.items():
        if key == "required":
            merged[key] = sorted(set(merged.get(key, [])) | set(value))
        elif key == "properties":
            merged[key] = {**merged.get(key, {}), **copy.deepcopy(value)}
        else:
            merged[key] = copy.deepcopy(value)
    return merged


def normalized_schema(document: dict[str, Any], schema: Any) -> dict[str, Any]:
    resolved = dereference(document, schema or {})
    if not isinstance(resolved, dict):
        return {}
    if not isinstance(resolved.get("allOf"), list):
        return resolved
    base = {
        key: copy.deepcopy(value) for key, value in resolved.items() if key != "allOf"
    }
    for part in resolved["allOf"]:
        base = _merge_schema(base, normalized_schema(document, part))
    return base


def _schema_type(schema: dict[str, Any]) -> str:
    declared = schema.get("type")
    if isinstance(declared, list):
        return "|".join(sorted(str(value) for value in declared))
    if declared is not None:
        return str(declared)
    if "properties" in schema or "additionalProperties" in schema:
        return "object"
    if "items" in schema:
        return "array"
    if "enum" in schema:
        return "string"
    return "unknown"


def _stable_values(values: list[Any]) -> list[Any]:
    return sorted(
        copy.deepcopy(values), key=lambda value: json.dumps(value, sort_keys=True)
    )


def schema_descriptor(
    document: dict[str, Any], raw_schema: Any, *, depth: int = 0
) -> dict[str, Any]:
    raw_reference = raw_schema.get("$ref") if isinstance(raw_schema, dict) else None
    schema = normalized_schema(document, raw_schema)
    descriptor: dict[str, Any] = {
        "type": _schema_type(schema),
        "nullable": bool(schema.get("nullable", False)),
    }
    if raw_reference is not None:
        descriptor["schema_ref"] = raw_reference
    for source_key, target_key in (
        ("format", "format"),
        ("minimum", "minimum"),
        ("maximum", "maximum"),
        ("exclusiveMinimum", "exclusive_minimum"),
        ("exclusiveMaximum", "exclusive_maximum"),
        ("minLength", "min_length"),
        ("maxLength", "max_length"),
        ("pattern", "pattern"),
        ("minItems", "min_items"),
        ("maxItems", "max_items"),
        ("uniqueItems", "unique_items"),
    ):
        if source_key in schema:
            descriptor[target_key] = copy.deepcopy(schema[source_key])
    if "enum" in schema:
        descriptor["enum"] = _stable_values(schema["enum"])
    if "default" in schema:
        descriptor["default"] = copy.deepcopy(schema["default"])
    if "items" in schema and depth < 8:
        descriptor["items"] = schema_descriptor(
            document, schema["items"], depth=depth + 1
        )
    for source_key, target_key in (("oneOf", "one_of"), ("anyOf", "any_of")):
        variants = schema.get(source_key)
        if isinstance(variants, list) and depth < 8:
            descriptor[target_key] = [
                _composition_descriptor(document, variant, depth=depth + 1)
                for variant in variants
            ]
    return descriptor


def _composition_descriptor(
    document: dict[str, Any], raw_schema: Any, *, depth: int
) -> dict[str, Any]:
    schema = normalized_schema(document, raw_schema)
    descriptor = schema_descriptor(document, raw_schema, depth=depth)
    if "required" in schema:
        descriptor["required_fields"] = sorted(schema["required"])
    return descriptor


def flatten_fields(
    document: dict[str, Any],
    raw_schema: Any,
    prefix: str = "",
    *,
    reference_stack: tuple[str, ...] = (),
) -> list[dict[str, Any]]:
    raw_reference = raw_schema.get("$ref") if isinstance(raw_schema, dict) else None
    if raw_reference is not None and raw_reference in reference_stack:
        return [{"path": prefix, "required": False, "recursive_ref": raw_reference}]
    next_stack = reference_stack + (
        (raw_reference,) if raw_reference is not None else ()
    )
    schema = normalized_schema(document, raw_schema)
    schema_type = _schema_type(schema)
    fields: list[dict[str, Any]] = []

    if schema_type == "object":
        required_names = set(schema.get("required", []))
        for name in sorted(schema.get("properties", {})):
            property_schema = schema["properties"][name]
            path = f"{prefix}.{name}" if prefix else name
            entry = {
                "path": path,
                "required": name in required_names,
                **schema_descriptor(document, property_schema),
            }
            fields.append(entry)
            property_type = entry["type"]
            if property_type == "object":
                fields.extend(
                    flatten_fields(
                        document,
                        property_schema,
                        path,
                        reference_stack=next_stack,
                    )
                )
            elif property_type == "array":
                item_schema = normalized_schema(document, property_schema).get("items")
                if item_schema is not None and _schema_type(
                    normalized_schema(document, item_schema)
                ) in {
                    "array",
                    "object",
                }:
                    fields.extend(
                        flatten_fields(
                            document,
                            item_schema,
                            f"{path}[]",
                            reference_stack=next_stack,
                        )
                    )
    elif schema_type == "array":
        item_schema = schema.get("items")
        if item_schema is not None and _schema_type(
            normalized_schema(document, item_schema)
        ) in {
            "array",
            "object",
        }:
            fields.extend(
                flatten_fields(
                    document,
                    item_schema,
                    f"{prefix}[]" if prefix else "[]",
                    reference_stack=next_stack,
                )
            )

    for composition_key in ("oneOf", "anyOf"):
        for variant_index, variant in enumerate(schema.get(composition_key, [])):
            for field in flatten_fields(
                document, variant, prefix, reference_stack=next_stack
            ):
                annotated = copy.deepcopy(field)
                annotated["composition"] = f"{composition_key}[{variant_index}]"
                if annotated not in fields:
                    fields.append(annotated)

    return sorted(
        fields, key=lambda field: (field.get("path", ""), field.get("composition", ""))
    )


def _parameter_contract(
    document: dict[str, Any], path_item: dict[str, Any], operation: dict[str, Any]
) -> list[dict[str, Any]]:
    parameters: dict[tuple[str, str], dict[str, Any]] = {}
    for raw_parameter in path_item.get("parameters", []) + operation.get(
        "parameters", []
    ):
        parameter = dereference(document, raw_parameter)
        key = (parameter["in"], parameter["name"])
        parameters[key] = parameter
    return [
        {
            "name": parameter["name"],
            "in": parameter["in"],
            "required": bool(parameter.get("required", False)),
            "schema": schema_descriptor(document, parameter.get("schema", {})),
        }
        for _, parameter in sorted(parameters.items())
    ]


def _content_schema(content: dict[str, Any]) -> tuple[str | None, Any]:
    if not content:
        return None, None
    media_type = (
        "application/json" if "application/json" in content else sorted(content)[0]
    )
    return media_type, content[media_type].get("schema")


def _request_body_contract(
    document: dict[str, Any], operation: dict[str, Any]
) -> dict[str, Any] | None:
    raw_body = operation.get("requestBody")
    if raw_body is None:
        return None
    body = dereference(document, raw_body)
    media_type, raw_schema = _content_schema(body.get("content", {}))
    if raw_schema is None:
        return {
            "required": bool(body.get("required", False)),
            "media_type": media_type,
            "schema": None,
            "fields": [],
        }
    return {
        "required": bool(body.get("required", False)),
        "media_type": media_type,
        "schema": schema_descriptor(document, raw_schema),
        "fields": flatten_fields(document, raw_schema),
    }


def _response_contract(
    document: dict[str, Any], operation: dict[str, Any]
) -> dict[str, Any]:
    statuses = sorted(
        int(status)
        for status in operation.get("responses", {})
        if status.isdigit() and 200 <= int(status) < 300
    )
    if not statuses:
        return {"status": None, "media_type": None, "schema": None, "fields": []}
    status = statuses[0]
    response = dereference(document, operation["responses"][str(status)])
    media_type, raw_schema = _content_schema(response.get("content", {}))
    return {
        "status": status,
        "media_type": media_type,
        "schema": schema_descriptor(document, raw_schema)
        if raw_schema is not None
        else None,
        "fields": flatten_fields(document, raw_schema)
        if raw_schema is not None
        else [],
    }


def operation_contract(
    document: dict[str, Any], path_item: dict[str, Any], operation: dict[str, Any]
) -> dict[str, Any]:
    return {
        "parameters": _parameter_contract(document, path_item, operation),
        "request_body": _request_body_contract(document, operation),
        "response": _response_contract(document, operation),
    }


def canonical_operation_map(document: dict[str, Any]) -> dict[str, dict[str, Any]]:
    operations: dict[str, dict[str, Any]] = {}
    for method, path, path_item, operation in iter_operations(document):
        operation_id = operation.get("operationId")
        if not operation_id:
            raise ValueError(
                f"official operation is missing operationId: {method} {path}"
            )
        if operation_id in operations:
            raise ValueError(f"duplicate official operationId: {operation_id}")
        operations[operation_id] = {
            "operation_id": operation_id,
            "tag": operation.get("tags", ["Uncategorized"])[0],
            "method": method,
            "path": path,
            "contract": operation_contract(document, path_item, operation),
        }
    return operations
