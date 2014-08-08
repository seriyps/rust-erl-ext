#!/usr/bin/env escript
%% See json_port.rs
%%
%% In this example we use `{packet, 2}`, so, unlike in erlang_rust_port, whole
%% response packet received at once. But this leads to more complex and less
%% performant Rust part (since we need to serialize to temporary in-memory
%% buffer to calculate packet's size).
-mode(compile).

main([]) ->
    main(["target/test/json_port"]);
main([PortPath]) ->
    AbsPortPath = filename:absname(PortPath),
    Port = erlang:open_port(
             {spawn_executable, AbsPortPath},
             [{packet, 2},
              binary]),

    run(Port, [<<"{\"array\": [1, -1, 0.1, {}, []], \"bool\": true,"
                 " \"null\": null, \"str\": \"Hello, world!\"}">>,
               <<"{true: true}">>,
               <<255, 0>>,
               "[\"not binary\"]"]),

    erlang:port_close(Port).

run(_, []) -> ok;
run(Port, [JsonBin | Examples]) ->
    Json = parse(Port, JsonBin),
    io:format("==========~nJson:~n'~tp'~nErlang:~n'~p'~n",
              [JsonBin, Json]),
    run(Port, Examples).

parse(Port, JsonBin) ->
    %% term_to_binary/1 actualy isn't required here, but if you want to
    %% implement json 'serialize', you may want to send, say
    %% `term_to_binary({parse, Bin})` and `term_to_binary({serialize, Term})`
    Port ! {self(), {command, term_to_binary(JsonBin)}},
    receive
        {Port, {data, Data}} ->
            binary_to_term(Data);
        Other ->
            {error, Other}
    end.
