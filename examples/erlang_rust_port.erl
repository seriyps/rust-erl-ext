#!/usr/bin/env escript
%% See erlang_rust_port.rs
%%
%% We don't use '{packet, N}' option, so in Rust side we can save some memory
%% by encoding directly to stdout (without calculating encoded data size).
%% The drawback of this solution is that in Erlang we should read data back
%% from port until it can be decoded by binary_to_term, and use binary
%% accumulator for that (see 'recv/2').
%% Alternatively, we can use '{packet, N}' option, and in Rust side first
%% encode terms to temporary MemWriter buffer, calculate and write it's size,
%% and then copy this buffer to stdout.
-mode(compile).

main([]) ->
    main(["target/test/erlang_rust_port"]);
main([PortPath]) ->
    AbsPortPath = filename:absname(PortPath),
    Port = erlang:open_port(
             {spawn_executable, AbsPortPath},
             [{args, ["-u", "-s", "-f"]},
              binary]),
    ok = loop(Port, 500),
    erlang:port_close(Port).


loop(_, 0) ->
    ok;
loop(Port, N) ->
    Term = #{"string" => atom,
             3.14 => Port,
             [] => {self(), -100000000000000000000000000},
             [1,2,3] => << <<0>> || _ <- lists:seq(1, 128) >>
            },
    %% send terms to port
    Port ! {self(), {command, term_to_binary(Term)}},
    %% receive them back
    case recv(Port, <<>>) of
        {ok, Term} ->                           % assert that term is the same
            io:format("."),
            loop(Port, N - 1);
        Other ->
            Other
    end.

recv(Port, Acc) ->
    receive
        {Port, {data, Data}} ->
            NewAcc = <<Acc/binary, Data/binary>>,
            try
                {ok, binary_to_term(NewAcc)}
            catch _:_ ->
                    recv(Port, NewAcc)
            end;
        Other ->
            Other
    end.
