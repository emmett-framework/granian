"""HTTP Range header parsing utilities according to RFC 7233."""

from typing import Optional


def parse_range_header(range_header: Optional[str]) -> Optional[list[tuple[Optional[int], Optional[int]]]]:
    """
    Parse HTTP Range header according to RFC 7233.

    Args:
        range_header: The Range header value (e.g., "bytes=0-499")

    Returns:
        List of (start, end) tuples where:
        - (start, end): Normal range from start to end (inclusive)
        - (start, None): Range from start to end of entity
        - (-suffix, None): Last suffix bytes of entity
        - None: Invalid or missing header

    Examples:
        >>> parse_range_header("bytes=0-499")
        [(0, 499)]
        >>> parse_range_header("bytes=500-")
        [(500, None)]
        >>> parse_range_header("bytes=-500")
        [(-500, None)]
        >>> parse_range_header("bytes=0-49,50-99")
        [(0, 49), (50, 99)]
    """
    if not range_header:
        return None

    range_header = range_header.strip()

    # Check if it starts with bytes= (case insensitive)
    if not range_header.lower().startswith('bytes='):
        return None

    # Remove the "bytes=" prefix
    ranges_str = range_header[6:].strip()

    if not ranges_str:
        return None

    # Split by comma for multiple ranges
    range_specs = [spec.strip() for spec in ranges_str.split(',')]

    ranges = []

    for spec in range_specs:
        if not spec:  # Empty range spec
            return None

        if '-' not in spec:
            return None

        # Count dashes to ensure exactly one
        if spec.count('-') != 1:
            return None

        # Split on the dash
        parts = spec.split('-', 1)
        start_str, end_str = parts

        start_str = start_str.strip()
        end_str = end_str.strip()

        try:
            if not start_str and not end_str:
                # Case: "-" (invalid)
                return None
            elif not start_str:
                # Case: "-500" (suffix range)
                suffix = int(end_str)
                if suffix <= 0:
                    return None
                ranges.append((-suffix, None))
            elif not end_str:
                # Case: "500-" (range from start to end)
                start = int(start_str)
                if start < 0:
                    return None
                ranges.append((start, None))
            else:
                # Case: "0-499" (normal range)
                start = int(start_str)
                end = int(end_str)
                if start < 0 or end < 0:
                    return None
                ranges.append((start, end))

        except ValueError:
            # Invalid integer
            return None

    return ranges if ranges else None
