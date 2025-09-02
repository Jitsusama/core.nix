#!/usr/bin/env ruby
# Claude Code statusline script - generates a stylish Unix-style prompt

require 'json'
require 'pathname'
require 'time'

module Claude
  module StatusLine
    module Domain
      # Value object representing a single displayable segment
      Segment = Struct.new(:content, :icon, :color, :bold, keyword_init: true) do
        def initialize(content:, icon: nil, color: nil, bold: false)
          super
        end
      end

      # Aggregate root - represents the complete status line
      class Line
        attr_reader :segments

        def initialize
          @segments = []
        end

        def <<(segment)
          segments << segment if segment
          self
        end
      end

      # Entity tracking token consumption with context awareness
      TokenConsumption = Struct.new(:input_tokens, :output_tokens, :context_limit, keyword_init: true) do
        # Claude Code considers context "low" around 80-85% usage
        DANGER_THRESHOLD = 85
        WARNING_THRESHOLD = 70
        # Claude Code reserves ~10% safety margin for its operations
        SAFETY_MARGIN = 0.90

        def context_percentage
          return 0 if input_tokens.nil? || input_tokens == 0 || context_limit == 0
          effective_limit = context_limit * SAFETY_MARGIN
          (input_tokens.to_f / effective_limit * 100).round
        end

        def raw_percentage
          return 0 if input_tokens.nil? || input_tokens == 0 || context_limit == 0
          (input_tokens.to_f / context_limit * 100).round
        end

        def context_severity
          case context_percentage
          when DANGER_THRESHOLD.. then :danger
          when WARNING_THRESHOLD.. then :warning
          else :normal
          end
        end
      end

      # Value object for parsed Claude session data
      ClaudeSession = Struct.new(:directory_name, :model_name, :transcript_path, :context_limit, keyword_init: true)
    end

    module Adapters
      # Primary adapter - parses Claude Code's JSON input
      class ClaudeInputParser
        DEFAULT_CONTEXT_LIMIT = 200_000

        def parse(json_input)
          Domain::ClaudeSession.new(
            directory_name: extract_directory_name(json_input),
            model_name: extract_model_name(json_input),
            transcript_path: json_input.dig('transcript_path'),
            context_limit: extract_context_limit(json_input)
          ).freeze
        end

        private

        def extract_directory_name(input)
          path = input.dig('workspace', 'current_dir') || 'unknown'
          Pathname.new(path).basename.to_s
        end

        def extract_model_name(input)
          model_id = input.dig('model', 'id') || 'unknown'
          return 'unknown' if model_id == 'unknown'

          # Format: claude-opus-4-1-20250805 -> opus-4.1
          match = model_id.match(/^claude-(?<name>[^-]+)-(?<version>.*?)-\d{8}$/)
          return model_id unless match

          name = match[:name]
          version = match[:version].tr('-', '.')

          version.empty? ? name : "#{name}-#{version}"
        end

        def extract_context_limit(input)
          # All current Claude models have 200k context
          # Could be extended if Claude Code starts passing this info
          DEFAULT_CONTEXT_LIMIT
        end
      end

      # Primary adapter - reads transcript files
      class TranscriptReader
        def read_token_consumption(path, context_limit)
          return nil unless path && File.exist?(path)

          messages = File.readlines(path)
            .map { |line| JSON.parse(line) rescue nil }
            .compact

          usage_history = messages
            .select { |msg| msg['type'] == 'assistant' }
            .select { |msg| msg.dig('message', 'usage') }
            .map { |msg| msg['message']['usage'] }

          return nil if usage_history.empty?

          # Input: most recent total (current context size)
          last_usage = usage_history.last
          input_total = (last_usage['input_tokens'] || 0) +
                        (last_usage['cache_creation_input_tokens'] || 0) +
                        (last_usage['cache_read_input_tokens'] || 0)

          # Output: sum of all outputs (total generated)
          output_total = usage_history.sum { |u| u['output_tokens'] || 0 }

          Domain::TokenConsumption.new(
            input_tokens: input_total,
            output_tokens: output_total,
            context_limit: context_limit,
          )
        rescue => e
          nil
        end
      end

      # Secondary adapter - renders to ANSI terminal
      class ANSITerminalRenderer
        ICONS = {
          folder: "\uF07B",
          model: "\uF5DC",
          database: "\uF1C0",
          download: "\uF019",
          upload: "\uF093",
          warning: "\uF071",
          fire: "\uF06D",
          check: "\uF058",
          pipe: "\u2502",
          separator: "\uE0B1"
        }.freeze

        COLORS = {
          yellow: '93',
          magenta: '95',
          cyan: '96',
          green: '92',
          red: '91',
          dark_blue: '34',
          white: '37',
          blue: '94'
        }.freeze

        def render(status_line)
          status_line.segments.map {
            |segment| format_segment(segment)
          }.join(" #{colorize(ICONS[:separator], COLORS[:blue])} ")
        end

        def format_composite(*parts)
          formatted_parts = parts.map do |part|
            icon = ICONS[part[:icon]] if part[:icon]
            color_code = COLORS[part[:color]] || part[:color]
            text = part[:text] || ''

            elements = []
            elements << colorize(icon, color_code) if icon
            elements << colorize(text, color_code) unless text.empty?
            elements.join(' ')
          end

          formatted_parts.join(' ')
        end

        private

        def format_segment(segment)
          parts = []

          if segment.icon
            icon_char = ICONS[segment.icon] || segment.icon
            parts << colorize(icon_char, color_code(segment.color))
          end

          content = segment.bold ? bold(segment.content) : segment.content
          parts << colorize(content, color_code(segment.color))

          parts.join(' ')
        end

        def color_code(color_key)
          COLORS[color_key] || color_key || COLORS[:white]
        end

        def colorize(text, code)
          "\033[#{code}m#{text}\033[0m"
        end

        def bold(text)
          "\033[1m#{text}\033[0m"
        end
      end
    end

    class Application
      attr_reader :session, :renderer, :transcript_reader

      def self.generate(json_input)
        new(json_input).generate
      end

      def initialize(json_input)
        @session = Adapters::ClaudeInputParser.new.parse(json_input)
        @renderer = Adapters::ANSITerminalRenderer.new
        @transcript_reader = Adapters::TranscriptReader.new
      end

      def generate
        status_line = Domain::Line.new
        status_line << directory_segment << model_segment

        if token_metrics
          status_line << context_segment
        end

        renderer.render(status_line)
      end

      private

      def token_metrics
        @token_metrics ||= transcript_reader.read_token_consumption(
          session.transcript_path,
          session.context_limit
        )
      end

      def directory_segment
        Domain::Segment.new(
          content: session.directory_name,
          icon: :folder,
          color: :yellow,
          bold: true
        )
      end

      def model_segment
        Domain::Segment.new(
          content: session.model_name,
          icon: :model,
          color: :magenta
        )
      end

      def context_segment
        severity_icon = case token_metrics.context_severity
        when :danger then :fire
        when :warning then :warning
        else :check
        end

        severity_color = case token_metrics.context_severity
        when :danger then :red
        when :warning then :yellow
        else :green
        end

        content = renderer.format_composite(
          {icon: :database, text: '', color: :cyan},
          {icon: severity_icon, text: "#{token_metrics.context_percentage}%", color: severity_color}
          {icon: :pipe, text: '', color: :dark_blue},
          {icon: :download, text: format_count(token_metrics.input_tokens), color: :green},
        )

        Domain::Segment.new(content: content)
      end

      def format_count(value)
        return "0" if value.nil? || value <= 0
        k_value = value / 1000.0
        k_value >= 10 ? "%.0fk" % k_value : "%.1fk" % k_value
      end
    end
  end
end

if __FILE__ == $0
  input = JSON.parse($stdin.read)
  puts Claude::StatusLine::Application.generate(input)
end
