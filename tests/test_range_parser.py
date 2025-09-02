from granian.utils.range import parse_range_header


class TestParseRangeHeader:
    """Test parsing of HTTP Range headers according to RFC 7233."""

    def test_no_header(self):
        """Test when no Range header is provided."""
        assert parse_range_header(None) is None
        assert parse_range_header('') is None

    def test_invalid_format(self):
        """Test invalid Range header formats."""
        assert parse_range_header('invalid') is None
        assert parse_range_header('bytes') is None
        assert parse_range_header('bytes=') is None
        assert parse_range_header('characters=0-100') is None

    def test_single_byte_range(self):
        """Test single byte range requests."""
        # Standard range
        result = parse_range_header('bytes=0-499')
        assert result == [(0, 499)]

        # Another standard range
        result = parse_range_header('bytes=500-999')
        assert result == [(500, 999)]

        # Single byte
        result = parse_range_header('bytes=0-0')
        assert result == [(0, 0)]

    def test_suffix_range(self):
        """Test suffix-byte-range-spec (last N bytes)."""
        # Last 500 bytes
        result = parse_range_header('bytes=-500')
        assert result == [(-500, None)]

        # Last byte
        result = parse_range_header('bytes=-1')
        assert result == [(-1, None)]

    def test_prefix_range(self):
        """Test byte-range-spec with no end (from N to end)."""
        # From byte 9500 to end
        result = parse_range_header('bytes=9500-')
        assert result == [(9500, None)]

        # From byte 0 to end
        result = parse_range_header('bytes=0-')
        assert result == [(0, None)]

    def test_multiple_ranges(self):
        """Test multiple byte ranges."""
        # Two ranges
        result = parse_range_header('bytes=0-49,50-99')
        assert result == [(0, 49), (50, 99)]

        # Three ranges
        result = parse_range_header('bytes=0-49,100-149,200-249')
        assert result == [(0, 49), (100, 149), (200, 249)]

        # Mix of different range types
        result = parse_range_header('bytes=0-49,-50')
        assert result == [(0, 49), (-50, None)]

        # First and last bytes
        result = parse_range_header('bytes=0-0,-1')
        assert result == [(0, 0), (-1, None)]

    def test_whitespace_handling(self):
        """Test Range headers with various whitespace."""
        # Spaces around ranges
        result = parse_range_header('bytes= 0-499 ')
        assert result == [(0, 499)]

        # Spaces around commas
        result = parse_range_header('bytes=0-49, 50-99')
        assert result == [(0, 49), (50, 99)]

        # Mixed whitespace
        result = parse_range_header('bytes= 0-49 , 100-149 , -50 ')
        assert result == [(0, 49), (100, 149), (-50, None)]

    def test_invalid_ranges(self):
        """Test invalid range specifications."""
        # Invalid numbers
        assert parse_range_header('bytes=abc-def') is None
        assert parse_range_header('bytes=0-abc') is None
        assert parse_range_header('bytes=abc-0') is None

        # Missing parts
        assert parse_range_header('bytes=-') is None
        assert parse_range_header('bytes=0--100') is None

        # Invalid separators
        assert parse_range_header('bytes=0:100') is None
        assert parse_range_header('bytes=0_100') is None

    def test_edge_cases(self):
        """Test edge cases and boundary conditions."""
        # Large numbers
        result = parse_range_header('bytes=0-9223372036854775807')
        assert result == [(0, 9223372036854775807)]

        # Mixed valid and invalid ranges (should return None for whole header)
        assert parse_range_header('bytes=0-49,invalid,100-149') is None

        # Empty range in multiple ranges
        assert parse_range_header('bytes=0-49,,100-149') is None

        # Just commas
        assert parse_range_header('bytes=,') is None

    def test_case_insensitive_bytes_unit(self):
        """Test that bytes unit is case insensitive."""
        result = parse_range_header('BYTES=0-499')
        assert result == [(0, 499)]

        result = parse_range_header('Bytes=0-499')
        assert result == [(0, 499)]

        result = parse_range_header('bYtEs=0-499')
        assert result == [(0, 499)]

    def test_real_world_examples(self):
        """Test real-world Range header examples."""
        # Video streaming - first chunk
        result = parse_range_header('bytes=0-1023')
        assert result == [(0, 1023)]

        # PDF page request
        result = parse_range_header('bytes=1024-2047')
        assert result == [(1024, 2047)]

        # Resume download from middle
        result = parse_range_header('bytes=2048-')
        assert result == [(2048, None)]

        # Get last part of file
        result = parse_range_header('bytes=-1024')
        assert result == [(-1024, None)]
